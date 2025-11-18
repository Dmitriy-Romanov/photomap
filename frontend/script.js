// Initialize map
const map = L.map('map').setView([52.5, 13.4], 10);

// Add tile layer
L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
}).addTo(map);

// Hide SVG path elements that look like flags
function hideSvgFlags() {
    // Hide SVG elements with flag class (most reliable method)
    const flagSvgs = document.querySelectorAll('svg.leaflet-attribution-flag');
    flagSvgs.forEach(svg => {
        svg.style.display = 'none';
        svg.style.width = '0px';
        svg.style.height = '0px';
        svg.style.visibility = 'hidden';
        svg.style.opacity = '0';
        svg.setAttribute('width', '0');
        svg.setAttribute('height', '0');
    });
}

// Run initially and when tiles are loaded
setTimeout(() => {
    hideSvgFlags();
}, 1000);

map.on('tileload', () => {
    hideSvgFlags();
});

// Run periodically to catch late-loading elements
setInterval(() => {
    hideSvgFlags();
}, 2000);

// Initialize marker cluster group
const markerClusterGroup = L.markerClusterGroup({
    iconCreateFunction: function(cluster) {
        const count = cluster.getChildCount();
        let size = 'small';
        let className = 'custom-cluster-icon';

        if (count >= 10) size = 'medium';
        if (count >= 50) size = 'large';

        const sizes = {
            small: 30,
            medium: 40,
            large: 50
        };

        return L.divIcon({
            html: '<div style="' +
                  'width: ' + sizes[size] + 'px; ' +
                  'height: ' + sizes[size] + 'px; ' +
                  'line-height: ' + sizes[size] + 'px; ' +
                  'border-radius: 50%; ' +
                  'background: #4285f4; ' +
                  'color: white; ' +
                  'text-align: center; ' +
                  'font-weight: bold; ' +
                  'font-size: ' + (sizes[size] * 0.4) + 'px; ' +
                  'border: 2px solid white; ' +
                  'box-shadow: 0 1px 3px rgba(0,0,0,0.3);' +
                  '">' + count + '</div>',
            className: className,
            iconSize: L.point(sizes[size], sizes[size])
        });
    },
    maxClusterRadius: 80,
    spiderfyOnMaxZoom: true,
    showCoverageOnHover: true,
    zoomToBoundsOnClick: true
});

// Load photo data from API
var photoData = [];

async function loadPhotos() {
    try {
        // Clear existing markers before loading new ones
        markerClusterGroup.clearLayers();
        photoData = [];

        const response = await fetch('/api/photos');
        photoData = await response.json();
        console.log(`Loaded ${photoData.length} photos from database`);
        addMarkers();
        return photoData; // Return the loaded data
    } catch (error) {
        console.error('Failed to load photos:', error);
    }
}

function createPhotoIcon(photo, useThumbnail = false) {
    const iconSize = useThumbnail ? 60 : 40;
    const apiUrl = useThumbnail ? '/api/thumbnail' : '/api/marker';

    return L.icon({
        iconUrl: apiUrl + '/' + photo.relative_path,
        iconSize: [iconSize, iconSize],
        iconAnchor: [iconSize/2, iconSize/2],
        popupAnchor: [0, -iconSize/2],
        className: 'thumbnail-icon'
    });
}

function addMarkers() {
    photoData.forEach(photo => {
        // Use thumbnail for better visibility when zoomed in
        const icon = createPhotoIcon(photo, true);

        const marker = L.marker([photo.lat, photo.lng], { icon: icon });

        const popupContent = `
            <div class="photo-popup">
                <img src="${photo.url}"
                     onerror="this.src='${photo.fallback_url}'"
                     alt="${photo.filename}" />
                <div class="filename">${photo.file_path}</div>
                <div class="datetime">${photo.datetime}</div>
            </div>
        `;

        marker.bindPopup(popupContent);
        markerClusterGroup.addLayer(marker);
    });

    // Add cluster group to map
    map.addLayer(markerClusterGroup);

    // Fit map to show all markers
    if (photoData.length > 0) {
        map.fitBounds(markerClusterGroup.getBounds(), { padding: [20, 20] });
    }

    // Update statistics
    updateStatistics();

    // Add zoom and move controls info
    map.on('zoomend', function() {
        console.log('Zoom ended, updating statistics');
        updateStatistics();
    });

    map.on('moveend', function() {
        console.log('Move ended, updating statistics');
        updateStatistics();
    });
}

function updateStatistics() {
    const totalPhotos = photoData.length;

    // Calculate visible photos by counting markers within current map bounds
    const bounds = map.getBounds();
    let visiblePhotos = 0;

    markerClusterGroup.eachLayer(function(layer) {
        const layerBounds = layer.getBounds ? layer.getBounds() : null;

        if (layerBounds && bounds.intersects(layerBounds)) {
            // Layer (cluster or marker) is within current view
            if (layer.getChildCount) {
                // It's a cluster - count all markers in it
                visiblePhotos += layer.getChildCount();
            } else {
                // It's a single marker
                visiblePhotos++;
            }
        } else if (!layerBounds && layer.getLatLng) {
            // Single marker without bounds - check if its position is in view
            const latlng = layer.getLatLng();
            if (bounds.contains(latlng)) {
                visiblePhotos++;
            }
        }
    });

    document.getElementById('total-photos').textContent = totalPhotos;
    document.getElementById('visible-photos').textContent = visiblePhotos;

    console.log('Statistics updated - Total:', totalPhotos, 'Visible:', visiblePhotos);
}

// Load settings when page loads
async function loadSettings() {
    try {
        const response = await fetch('/api/settings');
        const settings = await response.json();
        if (settings.last_folder) {
            document.getElementById('folder-input').value = settings.last_folder;
        } else {
            document.getElementById('folder-input').value = '';
        }
    } catch (error) {
        console.error('Failed to load settings:', error);
        document.getElementById('folder-input').value = '';
    }
}

// Initialize year range controls
function initializeYearControls() {
    const yearFromInput = document.getElementById('year-from');
    const yearToInput = document.getElementById('year-to');

    if (photoData.length === 0) {
        // No photos data yet, set default to current year
        const currentYear = new Date().getFullYear();
        yearFromInput.value = currentYear;
        yearToInput.value = currentYear;
        return;
    }

    // Extract years from photo dates
    console.log('Initializing year controls with', photoData.length, 'photos');
    console.log('Sample photo datetime:', photoData[0]?.datetime);

    const years = photoData
        .map(photo => {
            // Extract year from datetime (multiple formats supported)
            let year = null;

            // Pattern 1: Standard EXIF format "2021:05:22 20:21:21"
            let match = photo.datetime.match(/^(\d{4}):/);
            if (match) {
                year = parseInt(match[1]);
            } else {
                // Pattern 2: Alternative format "2021-05-22 20:21:21"
                match = photo.datetime.match(/^(\d{4})-/);
                if (match) {
                    year = parseInt(match[1]);
                } else {
                    // Pattern 3: Russian format "Дата съемки: 30.05.2025 11:04"
                    match = photo.datetime.match(/(\d{2})\.(\d{2})\.(\d{4})/);
                    if (match) {
                        year = parseInt(match[3]); // Third group is year in DD.MM.YYYY
                    } else {
                        // Pattern 4: Any 4-digit number (fallback)
                        match = photo.datetime.match(/(\d{4})/);
                        if (match) {
                            year = parseInt(match[1]);
                        }
                    }
                }
            }

            console.log('Photo:', photo.filename, 'datetime:', photo.datetime, 'extracted year:', year);
            return year;
        })
        .filter(year => year !== null);

    console.log('Valid years found:', years);

    if (years.length === 0) {
        // No valid dates found
        console.log('No valid years found, using current year');
        const currentYear = new Date().getFullYear();
        yearFromInput.value = currentYear;
        yearToInput.value = currentYear;
        return;
    }

    const minYear = Math.min(...years);
    const maxYear = Math.max(...years);

    // Set initial values
    yearFromInput.value = minYear;
    yearToInput.value = maxYear;

    // Set min/max attributes
    yearFromInput.min = minYear;
    yearFromInput.max = maxYear;
    yearToInput.min = minYear;
    yearToInput.max = maxYear;

    // Add event listeners for validation
    yearFromInput.addEventListener('change', function() {
        const fromValue = parseInt(this.value);
        const toValue = parseInt(yearToInput.value);

        // Ensure "From" is not greater than "To"
        if (fromValue > toValue) {
            this.value = toValue;
        }

        // Ensure "From" is not less than minYear
        if (fromValue < minYear) {
            this.value = minYear;
        }
    });

    yearToInput.addEventListener('change', function() {
        const fromValue = parseInt(yearFromInput.value);
        const toValue = parseInt(this.value);

        // Ensure "To" is not less than "From"
        if (toValue < fromValue) {
            this.value = fromValue;
        }

        // Ensure "To" is not greater than maxYear
        if (toValue > maxYear) {
            this.value = maxYear;
        }
    });

    console.log(`Year controls initialized: ${minYear} to ${maxYear}`);
}

// Load photos when page loads
loadSettings().then(() => {
    loadPhotos().then(() => {
        initializeYearControls();
    });
});

// === UI Control Functions ===

// Browse for folder and immediately start processing
async function processFolder() {
    const processButton = document.getElementById('process-button');
    const folderInput = document.getElementById('folder-input');
    const statusDiv = document.getElementById('processing-status');
    const statusText = document.getElementById('status-text');
    const progressText = document.getElementById('progress-text');

    // Get folder path from input
    const folderPath = folderInput.value.trim();

    // Validate folder path
    if (!folderPath) {
        showNotification('❌ Please enter a folder path', 'error');
        return;
    }

    try {
        // Disable button and show processing status
        processButton.disabled = true;
        processButton.textContent = '⏳ Processing...';
        statusDiv.style.display = 'block';
        statusText.textContent = 'Processing photos...';
        progressText.textContent = 'Analyzing folder...';

        // Step 1: Send folder path to server
        const response = await fetch('/api/set-folder', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ folder_path: folderPath })
        });

        const result = await response.json();

        if (result.status !== 'success') {
            throw new Error(result.message || 'Error setting folder');
        }

        showNotification(`✅ Folder set: ${folderPath}`, 'success');

        // Step 2: Start listening for SSE events
        const eventSource = new EventSource('/api/events');

        eventSource.onopen = async function() {
            showNotification('✅ SSE connection established', 'success');
            // Step 3: Initiate processing
            const processResponse = await fetch('/api/initiate-processing', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                }
            });

            const processResult = await processResponse.json();

            if (processResult.status !== 'started') {
                throw new Error(processResult.message || 'Error starting processing');
            }

            showNotification('✅ Processing initiated: ' + folderPath, 'success');
        };

        eventSource.onmessage = function(event) {
            const data = JSON.parse(event.data);
            if (data.event_type === 'processing_complete') {
                eventSource.close();
                statusDiv.style.display = 'none';
                processButton.disabled = false;
                processButton.textContent = '🚀 Process Photos';
                loadPhotos().then(() => {
                    initializeYearControls(); // Re-initialize year controls with new data
                }); // Refresh map
                updateStatistics();
                showNotification(`🎉 Processing completed! Found ${data.data.processed || 0} photos`, 'success');
            } else if (data.event_type === 'processing_error') {
                eventSource.close();
                statusDiv.style.display = 'none';
                processButton.disabled = false;
                processButton.textContent = '🚀 Process Photos';
                showNotification(`❌ Error: ${data.data.message}`, 'error');
            } else {
                // Handle other events like progress updates
                statusText.textContent = data.data.message || 'Processing...';
                if (data.data.total_files && data.data.processed) {
                    progressText.textContent = `${data.data.processed} / ${data.data.total_files}`;
                }
            }
        };

        eventSource.onerror = function() {
            eventSource.close();
            statusDiv.style.display = 'none';
            processButton.disabled = false;
            processButton.textContent = '🚀 Process Photos';
            showNotification('❌ Error connecting to the server for updates.', 'error');
        };

    } catch (error) {
        // Handle errors
        statusDiv.style.display = 'none';
        processButton.disabled = false;
        processButton.textContent = '🚀 Process Photos';
        showNotification('❌ Error: ' + error.message, 'error');
    }
}


// Show notification
function showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        z-index: 10000;
        padding: 15px 20px;
        border-radius: 5px;
        color: white;
        font-weight: bold;
        max-width: 300px;
        word-wrap: break-word;
        animation: slideIn 0.3s ease-out;
    `;

    if (type === 'success') {
        notification.style.background = '#28a745';
    } else if (type === 'error') {
        notification.style.background = '#dc3545';
    } else {
        notification.style.background = '#007bff';
    }

    notification.textContent = message;
    document.body.appendChild(notification);

    setTimeout(() => {
        notification.style.animation = 'slideOut 0.3s ease-in';
        setTimeout(() => {
            document.body.removeChild(notification);
        }, 300);
    }, 3000);
}

// Function to toggle info panel height
function toggleInfoWindow() {
    const windowContent = document.getElementById('window-content');
    const toggleButton = document.getElementById('toggle-window-btn');
    const floatingWindow = document.getElementById('floating-info-window');

    // Check current state
    if (windowContent.style.display === 'none') {
        // Expand the window
        windowContent.style.display = 'block';
        floatingWindow.style.height = 'auto';
        toggleButton.textContent = '⌄';

        // Save state to localStorage
        localStorage.setItem('infoWindowState', 'expanded');
    } else {
        // Collapse the window
        windowContent.style.display = 'none';
        floatingWindow.style.height = '26px';
        toggleButton.textContent = '⌃';

        // Save state to localStorage
        localStorage.setItem('infoWindowState', 'collapsed');
    }
    // No need to resize map - it's always full screen
}

// Attach event listener to the button after DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    const toggleButton = document.getElementById('toggle-window-btn');
    if (toggleButton) {
        toggleButton.addEventListener('click', toggleInfoWindow);
    }

    // Restore saved window state
    const savedState = localStorage.getItem('infoWindowState');
    const windowContent = document.getElementById('window-content');
    const floatingWindow = document.getElementById('floating-info-window');

    if (savedState === 'collapsed') {
        windowContent.style.display = 'none';
        floatingWindow.style.height = '26px';
        if (toggleButton) toggleButton.textContent = '⌃';
    }
});