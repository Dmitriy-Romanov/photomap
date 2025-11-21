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

// Update map center coordinates display
function updateMapCoordinates() {
    const center = map.getCenter();
    const coordsElement = document.getElementById('map-coordinates');
    if (coordsElement) {
        coordsElement.textContent = `Lat: ${center.lat.toFixed(5)}, Lon: ${center.lng.toFixed(5)}`;
    }
}

// Update coordinates on map move (real-time updates)
map.on('move', updateMapCoordinates);

// Initial update
updateMapCoordinates();

// Toggle coordinates and crosshair visibility
const coordsToggle = document.getElementById('coords-toggle');
const coordsElement = document.getElementById('map-coordinates');
const mapContainer = document.querySelector('.leaflet-container');

coordsToggle.addEventListener('change', function () {
    if (this.checked) {
        coordsElement.classList.remove('hidden');
        mapContainer.classList.remove('hide-crosshair');
    } else {
        coordsElement.classList.add('hidden');
        mapContainer.classList.add('hide-crosshair');
    }
});

// Initialize marker cluster group
const markerClusterGroup = L.markerClusterGroup({
    iconCreateFunction: function (cluster) {
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
    spiderfyOnMaxZoom: false,
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

        // Pre-calculate years for performance
        photoData.forEach(photo => {
            photo.year = getYearFromDatetime(photo.datetime);
        });

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
        iconAnchor: [iconSize / 2, iconSize / 2],
        popupAnchor: [0, -iconSize / 2],
        className: 'thumbnail-icon'
    });
}

function addMarkers() {
    photoData.forEach(photo => {
        // Use thumbnail for better visibility when zoomed in
        const icon = createPhotoIcon(photo, true);

        const marker = L.marker([photo.lat, photo.lng], {
            icon: icon,
            photoData: photo
        });

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
    map.on('zoomend', function () {
        console.log('Zoom ended, updating statistics');
        updateStatistics();
    });

    map.on('moveend', function () {
        console.log('Move ended, updating statistics');
        updateStatistics();
    });
}

function updateStatistics() {
    const totalPhotos = photoData.length;

    // Calculate visible photos by counting markers within current map bounds
    const bounds = map.getBounds();
    let visiblePhotos = 0;

    markerClusterGroup.eachLayer(function (layer) {
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

// Helper to extract year from datetime string
function getYearFromDatetime(datetime) {
    if (!datetime) return null;

    // Pattern 1: Standard EXIF format "2021:05:22 20:21:21"
    let match = datetime.match(/^(\d{4}):/);
    if (match) return parseInt(match[1]);

    // Pattern 2: Alternative format "2021-05-22 20:21:21"
    match = datetime.match(/^(\d{4})-/);
    if (match) return parseInt(match[1]);

    // Pattern 3: Russian format "Date taken: 30.05.2025 11:04"
    match = datetime.match(/(\d{2})\.(\d{2})\.(\d{4})/);
    if (match) return parseInt(match[3]);

    // Pattern 4: Any 4-digit number (fallback)
    match = datetime.match(/(\d{4})/);
    if (match) return parseInt(match[1]);

    return null;
}

// Filter markers based on year range
function filterMarkers() {
    const yearFromInput = document.getElementById('year-from');
    const yearToInput = document.getElementById('year-to');

    const fromYear = parseInt(yearFromInput.value);
    const toYear = parseInt(yearToInput.value);

    if (isNaN(fromYear) || isNaN(toYear)) return;

    console.log(`Filtering photos: ${fromYear} - ${toYear}`);

    // Clear existing markers
    markerClusterGroup.clearLayers();

    // Filter photos
    const filteredPhotos = photoData.filter(photo => {
        // Use pre-calculated year
        return photo.year !== null && photo.year >= fromYear && photo.year <= toYear;
    });

    console.log(`Found ${filteredPhotos.length} photos in range`);

    // Add filtered markers
    filteredPhotos.forEach(photo => {
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

    // Update statistics
    updateStatistics();
}

// Initialize year range controls
function initializeYearControls() {
    const yearFromInput = document.getElementById('year-from');
    const yearToInput = document.getElementById('year-to');

    if (photoData.length === 0) {
        const currentYear = new Date().getFullYear();
        yearFromInput.value = currentYear;
        yearToInput.value = currentYear;
        return;
    }

    // Extract years using pre-calculated value
    const years = photoData
        .map(photo => photo.year)
        .filter(year => year !== null);

    if (years.length === 0) {
        const currentYear = new Date().getFullYear();
        yearFromInput.value = currentYear;
        yearToInput.value = currentYear;
        const rangeInfo = document.getElementById('year-range-info');
        if (rangeInfo) rangeInfo.textContent = '';
        return;
    }

    const minYear = Math.min(...years);
    const maxYear = Math.max(...years);

    // Update info label
    const rangeInfo = document.getElementById('year-range-info');
    if (rangeInfo) {
        rangeInfo.textContent = `(${minYear}—${maxYear})`;
    }

    // Set initial values
    yearFromInput.value = minYear;
    yearToInput.value = maxYear;

    // Set min/max attributes
    yearFromInput.min = minYear;
    yearFromInput.max = maxYear;
    yearToInput.min = minYear;
    yearToInput.max = maxYear;

    // Add event listeners for validation and filtering
    yearFromInput.addEventListener('change', function () {
        let fromValue = parseInt(this.value);
        const toValue = parseInt(yearToInput.value);

        // Validation
        if (fromValue > toValue) {
            fromValue = toValue;
            this.value = toValue;
        }
        if (fromValue < minYear) {
            fromValue = minYear;
            this.value = minYear;
        }

        // Trigger filter
        filterMarkers();
    });

    yearToInput.addEventListener('change', function () {
        const fromValue = parseInt(yearFromInput.value);
        let toValue = parseInt(this.value);

        // Validation
        if (toValue < fromValue) {
            toValue = fromValue;
            this.value = fromValue;
        }
        if (toValue > maxYear) {
            toValue = maxYear;
            this.value = maxYear;
        }

        // Trigger filter
        filterMarkers();
    });

    // Initial filter (show all)
    // No need to call filterMarkers() here as addMarkers() already added everything

    console.log(`Year controls initialized: ${minYear} to ${maxYear}`);
}

// Shutdown application
async function shutdownApp() {
    if (!confirm('Are you sure you want to close PhotoMap?')) {
        return;
    }

    try {
        const response = await fetch('/api/shutdown', {
            method: 'POST'
        });

        if (response.ok) {
            showNotification('👋 Server stopped. Closing...', 'success');
            setTimeout(() => {
                window.close();
                // Fallback if window.close() is blocked
                document.body.innerHTML = '<div style="display:flex;justify-content:center;align-items:center;height:100vh;flex-direction:column;font-family:sans-serif;"><h1>👋 PhotoMap is closed</h1><p>You can close this tab now.</p></div>';
            }, 1000);
        } else {
            showNotification('❌ Failed to stop server', 'error');
        }
    } catch (error) {
        console.error('Shutdown error:', error);
        showNotification('❌ Error stopping server', 'error');
    }
}

// Load photos when page loads
loadSettings().then(() => {
    loadPhotos().then(() => {
        initializeYearControls();
    });
});

// === UI Control Functions ===

// Open native folder selection dialog
async function openFolderDialog() {
    const openButton = document.getElementById('open-button');
    const folderInput = document.getElementById('folder-input');

    try {
        openButton.disabled = true;
        openButton.textContent = '⏳...';

        const response = await fetch('/api/select-folder', {
            method: 'POST'
        });

        const result = await response.json();

        if (result.status === 'success') {
            folderInput.value = result.folder_path;
            showNotification('✅ Folder selected', 'success');

            // Auto-start processing
            console.log('Auto-starting processing...');
            await processFolder();
        } else if (result.status === 'cancelled') {
            // User cancelled, do nothing
            console.log('Folder selection cancelled');
        } else {
            showNotification('❌ ' + (result.message || 'Error selecting folder'), 'error');
        }
    } catch (error) {
        console.error('Error selecting folder:', error);
        showNotification('❌ Error selecting folder', 'error');
    } finally {
        openButton.disabled = false;
        openButton.textContent = '📂 Open';
    }
}

// Browse for folder and immediately start processing
async function processFolder() {
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
        // Show processing status
        statusDiv.style.display = 'flex';
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

        eventSource.onopen = async function () {
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

        eventSource.onmessage = function (event) {
            const data = JSON.parse(event.data);
            if (data.event_type === 'processing_complete') {
                eventSource.close();
                statusDiv.style.display = 'none';
                loadPhotos().then(() => {
                    initializeYearControls(); // Re-initialize year controls with new data
                }); // Refresh map
                updateStatistics();
                showNotification(`🎉 Processing completed! Found ${data.data.processed || 0} photos`, 'success');
            } else if (data.event_type === 'processing_error') {
                eventSource.close();
                statusDiv.style.display = 'none';
                showNotification(`❌ Error: ${data.data.message}`, 'error');
            } else {
                // Handle other events like progress updates
                statusText.textContent = data.data.message || 'Processing...';
                if (data.data.total_files && data.data.processed) {
                    progressText.textContent = `${data.data.processed} / ${data.data.total_files}`;
                }
            }
        };

        eventSource.onerror = function () {
            eventSource.close();
            statusDiv.style.display = 'none';
            showNotification('❌ Error connecting to the server for updates.', 'error');
        };

    } catch (error) {
        // Handle errors
        statusDiv.style.display = 'none';
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
    const panel = document.getElementById('floating-info-window');
    const toggleButton = document.getElementById('toggle-window-btn');

    // Check if currently collapsed
    const isCollapsed = panel.classList.contains('collapsed');

    if (isCollapsed) {
        // Expand
        panel.classList.remove('collapsed');
        toggleButton.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>';
        toggleButton.title = "Minimize";
        localStorage.setItem('infoWindowState', 'expanded');
    } else {
        // Collapse
        panel.classList.add('collapsed');
        toggleButton.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 15 12 9 18 15"></polyline></svg>';
        toggleButton.title = "Expand";
        localStorage.setItem('infoWindowState', 'collapsed');
    }
}

// Attach event listener to the button after DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    const toggleButton = document.getElementById('toggle-window-btn');
    // Event listener removed to avoid double-toggling (onclick is in HTML)

    // Restore saved window state
    const savedState = localStorage.getItem('infoWindowState');
    const panel = document.getElementById('floating-info-window');

    if (savedState === 'collapsed') {
        panel.classList.add('collapsed');
        if (toggleButton) {
            toggleButton.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 15 12 9 18 15"></polyline></svg>';
            toggleButton.title = "Expand";
        }
    }
});

// === Cluster Gallery Logic ===

// Handle cluster clicks
markerClusterGroup.on('clusterclick', function (a) {
    const cluster = a.layer;
    const childCount = cluster.getChildCount();
    const markers = cluster.getAllChildMarkers();

    // Check if all markers are at the exact same location
    let allSameLocation = true;
    if (markers.length > 0) {
        const firstLatLng = markers[0].getLatLng();
        for (let i = 1; i < markers.length; i++) {
            if (!markers[i].getLatLng().equals(firstLatLng)) {
                allSameLocation = false;
                break;
            }
        }
    }

    // If markers are spread out and we can still zoom in, let the map zoom
    if (!allSameLocation && map.getZoom() < map.getMaxZoom()) {
        a.layer.zoomToBounds();
        return;
    }

    // If we are here, it means either:
    // 1. All markers are at the same location
    // 2. We are at max zoom

    // If cluster is small, just spiderfy (expand) as usual
    if (childCount < 10) {
        cluster.spiderfy();
    } else {
        // If cluster is large, open the gallery modal
        openClusterGallery(cluster);
    }
});

// Gallery State
let galleryState = {
    photos: [],
    currentPage: 1,
    itemsPerPage: 28
};

// Open the cluster gallery modal
function openClusterGallery(cluster) {
    const markers = cluster.getAllChildMarkers();
    const photos = markers.map(marker => {
        return marker.options.photoData;
    });

    // Initialize state
    galleryState.photos = photos;
    galleryState.currentPage = 1;

    const modal = document.getElementById('cluster-modal');
    const title = document.getElementById('cluster-title');

    // Update title
    title.textContent = `${photos.length} Photos in this location`;

    // Render first page
    renderGalleryPage(1);

    // Show Grid View, Hide Detail View
    showClusterGrid();

    // Show Modal
    modal.classList.remove('hidden');
    modal.style.display = 'flex'; // Ensure flex display

    // Prevent background scrolling
    document.body.style.overflow = 'hidden';
}

// Render specific page of gallery
function renderGalleryPage(page) {
    const grid = document.getElementById('cluster-grid');
    const pagination = document.getElementById('cluster-pagination');
    const prevBtn = document.getElementById('pagination-prev');
    const nextBtn = document.getElementById('pagination-next');
    const pageInfo = document.getElementById('pagination-info');

    // Calculate slice
    const start = (page - 1) * galleryState.itemsPerPage;
    const end = start + galleryState.itemsPerPage;
    const pagePhotos = galleryState.photos.slice(start, end);
    const totalPages = Math.ceil(galleryState.photos.length / galleryState.itemsPerPage);

    // Update state
    galleryState.currentPage = page;

    // Clear existing grid
    grid.innerHTML = '';

    // Populate grid
    pagePhotos.forEach(photo => {
        if (!photo) return; // Safety check

        const thumb = document.createElement('div');
        thumb.className = 'cluster-thumbnail';
        thumb.onclick = () => showPhotoInGallery(photo);

        const img = document.createElement('img');
        img.src = `/api/thumbnail/${photo.relative_path}`;
        img.alt = photo.filename;
        img.loading = 'lazy';

        thumb.appendChild(img);
        grid.appendChild(thumb);
    });

    // Update Pagination Controls
    if (totalPages > 1) {
        pagination.classList.remove('hidden');
        pageInfo.textContent = `Page ${page} of ${totalPages}`;
        prevBtn.disabled = page === 1;
        nextBtn.disabled = page === totalPages;
    } else {
        pagination.classList.add('hidden');
    }

    // Scroll grid to top
    grid.scrollTop = 0;
}

// Change gallery page
function changeGalleryPage(delta) {
    const newPage = galleryState.currentPage + delta;
    const totalPages = Math.ceil(galleryState.photos.length / galleryState.itemsPerPage);

    if (newPage >= 1 && newPage <= totalPages) {
        renderGalleryPage(newPage);
    }
}

// Close the cluster gallery modal
function closeClusterModal() {
    const modal = document.getElementById('cluster-modal');
    modal.classList.add('hidden');

    // Allow background scrolling again
    document.body.style.overflow = '';

    setTimeout(() => {
        modal.style.display = 'none';
    }, 300);
}

// Show specific photo in Detail View
function showPhotoInGallery(photo) {
    const gridView = document.getElementById('cluster-grid-view');
    const detailView = document.getElementById('cluster-detail-view');
    const backBtn = document.getElementById('cluster-back-btn');

    const detailImg = document.getElementById('cluster-detail-img');
    const detailFilename = document.getElementById('cluster-detail-filename');
    const detailDate = document.getElementById('cluster-detail-date');

    // Update content
    detailImg.src = photo.url;
    detailImg.onerror = () => { detailImg.src = photo.fallback_url; };
    detailFilename.textContent = photo.filename;
    detailDate.textContent = photo.datetime;

    // Switch views
    gridView.classList.add('hidden');
    detailView.classList.remove('hidden');
    backBtn.classList.remove('hidden');
}

// Switch back to Grid View
function showClusterGrid() {
    const gridView = document.getElementById('cluster-grid-view');
    const detailView = document.getElementById('cluster-detail-view');
    const backBtn = document.getElementById('cluster-back-btn');

    // Switch views
    detailView.classList.add('hidden');
    gridView.classList.remove('hidden');
    backBtn.classList.add('hidden');
}

// Close modal when clicking outside content
document.getElementById('cluster-modal').addEventListener('click', function (e) {
    if (e.target === this) {
        closeClusterModal();
    }
});

// Keyboard navigation
document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
        const modal = document.getElementById('cluster-modal');
        if (!modal.classList.contains('hidden')) {
            // If in detail view, go back to grid
            const detailView = document.getElementById('cluster-detail-view');
            if (!detailView.classList.contains('hidden')) {
                showClusterGrid();
            } else {
                closeClusterModal();
            }
        }
    }
});