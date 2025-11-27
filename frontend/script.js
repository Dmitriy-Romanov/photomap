// Initialize map with OS-specific scroll wheel zoom settings
const isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;

const map = L.map('map', {
    scrollWheelZoom: true,
    // macOS: faster zoom (full steps), Windows: slower zoom (quarter steps)
    wheelPxPerZoomLevel: isMac ? 120 : 240,
    zoomSnap: isMac ? 1.0 : 0.25,
    zoomDelta: isMac ? 1.0 : 0.25
}).setView([52.5, 13.4], 10);

// Add tile layer
L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
}).addTo(map);

// Add user location marker
let userLocationMarker = null;
let userLocation = null; // Store user's coordinates

map.locate({ setView: false, maxZoom: 16 });

map.on('locationfound', function (e) {
    const radius = e.accuracy / 2;

    // Save user location
    userLocation = e.latlng;

    // Remove old marker if exists
    if (userLocationMarker) {
        map.removeLayer(userLocationMarker);
    }

    // Create custom icon for user location (green)
    const userIcon = L.divIcon({
        className: 'user-location-marker',
        html: '<div class="user-location-icon"></div>',
        iconSize: [22, 22],
        iconAnchor: [11, 11]
    });

    // Add marker with compact popup
    userLocationMarker = L.marker(e.latlng, { icon: userIcon }).addTo(map)
        .bindPopup(`📍 Your location (±${Math.round(radius)}m)`, {
            className: 'compact-popup'
        });

    // Optional: add accuracy circle (green)
    L.circle(e.latlng, radius, {
        color: '#34A853',
        fillColor: '#34A853',
        fillOpacity: 0.1,
        weight: 1
    }).addTo(map);
});

map.on('locationerror', function (e) {
    console.log('Geolocation access denied or unavailable:', e.message);
});

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

    // Sync coordinates to experimental panel
    const expCoords = document.getElementById('exp-coordinates');
    if (expCoords) {
        expCoords.textContent = `Lat: ${center.lat.toFixed(5)}, Lon: ${center.lng.toFixed(5)}`;
    }
}

// Bind coordinate updates to map events
map.on('move', updateMapCoordinates);
map.on('zoom', updateMapCoordinates);

// Sync toggle helper
function syncToggles(mainId, expId, callback) {
    const main = document.getElementById(mainId);
    const exp = document.getElementById(expId);

    if (!main) return;

    function update(checked, source) {
        if (source !== main) main.checked = checked;
        if (exp && source !== exp) exp.checked = checked;
        if (callback) callback(checked);
    }

    main.addEventListener('change', (e) => update(e.target.checked, main));
    if (exp) {
        exp.addEventListener('change', (e) => update(e.target.checked, exp));
        // Initial sync from main to exp
        exp.checked = main.checked;
    }
}

// Initialize syncs
document.addEventListener('DOMContentLoaded', () => {
    // Coordinates Toggle
    // Note: The toggle ID in HTML is 'exp-map-coords-toggle'
    const expCoordsToggle = document.getElementById('exp-map-coords-toggle');
    const expCoordsDisplay = document.getElementById('exp-coordinates'); // Correct ID from HTML

    if (expCoordsToggle) {
        expCoordsToggle.addEventListener('change', (e) => {
            if (e.target.checked) {
                if (expCoordsDisplay) expCoordsDisplay.style.visibility = 'visible';
                map.getContainer().classList.remove('hide-crosshair');
            } else {
                if (expCoordsDisplay) expCoordsDisplay.style.visibility = 'hidden';
                map.getContainer().classList.add('hide-crosshair');
            }
        });
        // Initial state
        const isChecked = expCoordsToggle.checked;
        if (expCoordsDisplay) expCoordsDisplay.style.visibility = isChecked ? 'visible' : 'hidden';
        if (!isChecked) {
            map.getContainer().classList.add('hide-crosshair');
        }
    }

    // Routes Toggle
    const expRoutesToggle = document.getElementById('exp-routes-toggle');
    if (expRoutesToggle) {
        expRoutesToggle.addEventListener('change', () => {
            toggleRoutes(); // Call the function directly
        });
    }

    // Heatmap Toggle
    const expHeatmapToggle = document.getElementById('exp-heatmap-toggle');
    if (expHeatmapToggle) {
        expHeatmapToggle.addEventListener('change', () => {
            // We need to trigger the filter/update logic
            // If we have filtered markers, we should use them, otherwise all photos
            // But filterMarkers() handles the heatmap toggle check internally!
            // So we just need to re-run filterMarkers() or updateStatistics/addMarkers logic.
            // Let's call filterMarkers() as it seems to be the central place for deciding what to show.
            filterMarkers();
        });
    }

    // Browser Autostart Toggle
    const expAutostartToggle = document.getElementById('exp-browser-autostart-toggle');
    if (expAutostartToggle) {
        expAutostartToggle.addEventListener('change', async (e) => {
            const startBrowser = e.target.checked;
            try {
                const getResponse = await fetch('/api/settings');
                const currentSettings = await getResponse.json();
                const newSettings = { ...currentSettings, start_browser: startBrowser };

                const response = await fetch('/api/settings', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(newSettings)
                });

                if (response.ok) {
                    showNotification('✅ Settings saved', 'success');
                } else {
                    throw new Error('Failed to save');
                }
            } catch (error) {
                console.error('Error updating settings:', error);
                showNotification('❌ Failed to save settings', 'error');
                e.target.checked = !startBrowser;
            }
        });
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
            html: '<div class="cluster-icon-inner" style="' +
                'width: ' + sizes[size] + 'px; ' +
                'height: ' + sizes[size] + 'px; ' +
                'line-height: ' + sizes[size] + 'px; ' +
                'font-size: ' + (sizes[size] * 0.4) + 'px;' +
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

// Layer group for travel lines
const routesLayerGroup = L.layerGroup().addTo(map);

// Heatmap layer
let heatLayer = null;

// Load photo data from API
let photoData = [];

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

function createPopupContent(photo) {
    return `
        <div class="photo-popup">
            <img src="${photo.url}"
                 onerror="this.src='${photo.fallback_url}'"
                 alt="${photo.filename}" />
            <div class="filename">${photo.file_path}</div>
            <div class="datetime">${photo.datetime}</div>
        </div>
    `;
}

function addMarkers() {
    // Check if heatmap mode is enabled
    const heatmapToggle = document.getElementById('exp-heatmap-toggle');
    if (heatmapToggle && heatmapToggle.checked) {
        // Use heatmap instead of markers (routes not compatible with heatmap)
        updateHeatmap(photoData);
        updateStatistics();
        return;
    }

    // Normal marker mode
    photoData.forEach(photo => {
        // Use thumbnail for better visibility when zoomed in
        const icon = createPhotoIcon(photo, true);

        const marker = L.marker([photo.lat, photo.lng], {
            icon: icon,
            photoData: photo
        });

        marker.bindPopup(createPopupContent(photo));
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

    // Draw routes on initial load if enabled
    drawPolylines();

    // Add zoom and move controls info
    map.on('zoomend', function () {
        console.log('Zoom ended, updating statistics');
        updateStatistics();
        map.on('moveend', function () {
            console.log('Move ended, updating statistics');
            updateStatistics();
        });

        // Draw routes if enabled
        drawPolylines();
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

    // Update experimental panel stats
    const expTotal = document.getElementById('exp-total-photos');
    const expVisible = document.getElementById('exp-visible-photos');

    if (expTotal) expTotal.textContent = totalPhotos;
    if (expVisible) expVisible.textContent = visiblePhotos;

    console.log('Statistics updated - Total:', totalPhotos, 'Visible:', visiblePhotos);
}



// Make element draggable
function makeDraggable(element) {
    let pos1 = 0, pos2 = 0, pos3 = 0, pos4 = 0;

    element.onmousedown = dragMouseDown;

    function dragMouseDown(e) {
        // Don't drag if clicking on inputs or buttons
        if (['INPUT', 'BUTTON', 'LABEL', 'A', 'SELECT', 'TEXTAREA'].includes(e.target.tagName) ||
            e.target.closest('button') || e.target.closest('label')) {
            return;
        }

        e = e || window.event;
        e.preventDefault();
        // get the mouse cursor position at startup:
        pos3 = e.clientX;
        pos4 = e.clientY;
        document.onmouseup = closeDragElement;
        // call a function whenever the cursor moves:
        document.onmousemove = elementDrag;
    }

    function elementDrag(e) {
        e = e || window.event;
        e.preventDefault();
        // calculate the new cursor position:
        pos1 = pos3 - e.clientX;
        pos2 = pos4 - e.clientY;
        pos3 = e.clientX;
        pos4 = e.clientY;

        // Calculate new position
        let newTop = element.offsetTop - pos2;
        let newLeft = element.offsetLeft - pos1;

        // Get panel dimensions
        const panelWidth = element.offsetWidth;
        const panelHeight = element.offsetHeight;

        // Get viewport dimensions
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        // Constrain to viewport boundaries
        // Top: must be >= 0
        if (newTop < 0) newTop = 0;
        // Left: must be >= 0
        if (newLeft < 0) newLeft = 0;
        // Bottom: panel bottom edge must be <= viewport height
        if (newTop + panelHeight > viewportHeight) {
            newTop = viewportHeight - panelHeight;
        }
        // Right: panel right edge must be <= viewport width
        if (newLeft + panelWidth > viewportWidth) {
            newLeft = viewportWidth - panelWidth;
        }

        // Set the element's new position:
        element.style.top = newTop + "px";
        element.style.left = newLeft + "px";
    }

    function closeDragElement() {
        // stop moving when mouse button is released:
        document.onmouseup = null;
        document.onmousemove = null;
    }
}

// Initialize draggable panel
document.addEventListener('DOMContentLoaded', () => {
    const panel = document.getElementById('experimental-panel');
    if (panel) {
        makeDraggable(panel);

        // Double-click to reset position
        panel.addEventListener('dblclick', (e) => {
            // Don't reset if double-clicking on inputs, buttons, etc.
            if (['INPUT', 'BUTTON', 'LABEL', 'A', 'SELECT', 'TEXTAREA'].includes(e.target.tagName) ||
                e.target.closest('button') || e.target.closest('label')) {
                return;
            }

            // Reset to default position
            panel.style.top = '12px';
            panel.style.left = '52px';
            showNotification('🔄 Panel reset to default position', 'info');
        });
    }
});

// Minimize/Close buttons
document.getElementById('minimize-btn')?.addEventListener('click', toggleExpPanel);
document.getElementById('close-btn')?.addEventListener('click', async () => {
    if (confirm('Are you sure you want to close PhotoMap?')) {
        try {
            // 1. Get current panel position
            const panel = document.getElementById('experimental-panel');
            let top = 12;
            let left = 52;

            if (panel) {
                const rect = panel.getBoundingClientRect();
                top = Math.round(rect.top);
                left = Math.round(rect.left);
            }

            // 2. Fetch current settings to avoid overwriting other fields
            const settingsResponse = await fetch('/api/settings');
            if (settingsResponse.ok) {
                const currentSettings = await settingsResponse.json();

                // 3. Update settings
                currentSettings.top = top;
                currentSettings.left = left;

                // 4. Save settings
                const updateResponse = await fetch('/api/update_settings', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify(currentSettings)
                });

                if (!updateResponse.ok) {
                    console.error('Failed to save settings before shutdown');
                } else {
                    const result = await updateResponse.json();
                }
            } else {
                console.error('Failed to fetch current settings');
            }

            // 5. Shutdown - wait 300ms to ensure settings are written to disk
            setTimeout(async () => {
                try {
                    await fetch('/api/shutdown', { method: 'POST' });
                    document.body.innerHTML = '<div class="shutdown-screen"><h1>PhotoMap is closed</h1></div>';
                } catch (e) {
                    console.error('Shutdown request failed', e);
                }
            }, 300);
        } catch (e) {
            console.error('Shutdown failed', e);
        }
    }
});



// Load settings when page loads
async function loadSettings() {
    try {
        const response = await fetch('/api/settings');
        const settings = await response.json();

        // Load folders from settings (new multi-folder support)
        if (settings.folders && Array.isArray(settings.folders)) {
            const folders = settings.folders.filter(f => f !== null && f !== "");

            if (folders.length > 0) {
                const input = document.getElementById('exp-folder-input');
                if (input) {
                    if (folders.length > 1) {
                        input.value = `Multiple folders (${folders.length})`;
                    } else {
                        input.value = folders[0];
                    }
                }

                // Store folders for processing
                window.selectedFolders = folders;
                console.log(`Loaded ${folders.length} folder(s) from settings:`, folders);
            }
        } else if (settings.last_folder) {
            // Backward compatibility with old single folder
            const input = document.getElementById('exp-folder-input');
            if (input) input.value = settings.last_folder;
            window.selectedFolders = [settings.last_folder];
        }

        // Set browser autostart toggle
        const browserAutostartToggle = document.getElementById('exp-browser-autostart-toggle');
        if (browserAutostartToggle) {
            browserAutostartToggle.checked = settings.start_browser !== undefined ? settings.start_browser : true;
        }

        // Set map coordinates toggle
        const mapCoordsToggle = document.getElementById('exp-map-coords-toggle');
        if (mapCoordsToggle) {
            mapCoordsToggle.checked = settings.map_coords !== undefined ? settings.map_coords : true;
            // Trigger change event to apply UI state (coordinates visibility, crosshair)
            mapCoordsToggle.dispatchEvent(new Event('change'));
        }

        // Set routes toggle (don't trigger - will apply when data loads)
        const routesToggle = document.getElementById('exp-routes-toggle');
        if (routesToggle) {
            routesToggle.checked = settings.routes !== undefined ? settings.routes : false;
        }

        // Set heatmap toggle (don't trigger - will apply when data loads)
        const heatmapToggle = document.getElementById('exp-heatmap-toggle');
        if (heatmapToggle) {
            heatmapToggle.checked = settings.heatmap !== undefined ? settings.heatmap : false;
        }

        // Apply panel position
        const panel = document.getElementById('experimental-panel');
        if (panel) {
            let top = settings.top !== undefined ? settings.top : 12;
            let left = settings.left !== undefined ? settings.left : 52;

            // Check if panel would be outside viewport - reset to defaults if so
            // We need to get panel dimensions after it's rendered
            setTimeout(() => {
                const panelWidth = panel.offsetWidth;
                const panelHeight = panel.offsetHeight;
                const viewportWidth = window.innerWidth;
                const viewportHeight = window.innerHeight;

                // Check boundaries
                let needsReset = false;
                if (top < 0 || left < 0) {
                    needsReset = true;
                } else if (top + panelHeight > viewportHeight || left + panelWidth > viewportWidth) {
                    needsReset = true;
                }

                if (needsReset) {
                    console.log('Panel position outside viewport, resetting to defaults');
                    panel.style.top = '12px';
                    panel.style.left = '52px';
                } else {
                    panel.style.top = `${top}px`;
                    panel.style.left = `${left}px`;
                }
            }, 0);
        }
    } catch (error) {
        console.error('Failed to load settings:', error);
    }
}

// Browser autostart toggle logic
document.addEventListener('DOMContentLoaded', () => {
    const browserAutostartToggle = document.getElementById('browser-autostart-toggle');
    if (browserAutostartToggle) {
        browserAutostartToggle.addEventListener('change', async function () {
            const startBrowser = this.checked;
            try {
                // We need to fetch current settings first to preserve other fields (like last_folder)
                // Or the backend could handle partial updates, but our backend replaces the whole struct.
                // Let's fetch first.
                const getResponse = await fetch('/api/settings');
                const currentSettings = await getResponse.json();

                const newSettings = {
                    ...currentSettings,
                    start_browser: startBrowser
                };

                const response = await fetch('/api/settings', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify(newSettings)
                });

                if (!response.ok) {
                    throw new Error('Failed to update settings');
                }
                console.log('Browser autostart setting updated:', startBrowser);
                showNotification('✅ Settings saved', 'success');
            } catch (error) {
                console.error('Error updating settings:', error);
                showNotification('❌ Failed to save settings', 'error');
                // Revert toggle if failed
                this.checked = !startBrowser;
            }
        });
    }
});

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
    const yearFromInput = document.getElementById('exp-year-from');
    const yearToInput = document.getElementById('exp-year-to');

    const fromYear = parseInt(yearFromInput.value);
    const toYear = parseInt(yearToInput.value);

    if (isNaN(fromYear) || isNaN(toYear)) return;

    console.log(`Filtering photos: ${fromYear} - ${toYear}`);

    // Clear existing markers
    // Clear existing markers
    markerClusterGroup.clearLayers();
    routesLayerGroup.clearLayers();
    if (heatLayer) {
        map.removeLayer(heatLayer);
    }

    // Filter photos
    const filteredPhotos = photoData.filter(photo => {
        // Use pre-calculated year
        return photo.year !== null && photo.year >= fromYear && photo.year <= toYear;
    });

    console.log(`Found ${filteredPhotos.length} photos in range`);

    // Update statistics immediately
    updateStatistics();

    // Update again after a short delay to ensure cluster animations/updates are done
    // This fixes the issue where visible count stays 0 until map move
    setTimeout(updateStatistics, 100);

    // Check if heatmap is enabled
    const heatmapToggle = document.getElementById('exp-heatmap-toggle');
    if (heatmapToggle && heatmapToggle.checked) {
        updateHeatmap(filteredPhotos);
    } else {
        // Add filtered markers
        filteredPhotos.forEach(photo => {
            const icon = createPhotoIcon(photo, true);
            const marker = L.marker([photo.lat, photo.lng], { icon: icon });

            marker.bindPopup(createPopupContent(photo));
            markerClusterGroup.addLayer(marker);
        });

        // Add cluster group to map
        map.addLayer(markerClusterGroup);

        // Redraw routes (only if heatmap is OFF, usually looks better)
        drawPolylines();
    }
}

// Initialize year range controls
function initializeYearControls() {
    const expYearFrom = document.getElementById('exp-year-from');
    const expYearTo = document.getElementById('exp-year-to');
    const expRangeLabel = document.getElementById('exp-year-range-label');

    if (photoData.length === 0) {
        const currentYear = new Date().getFullYear();
        if (expYearFrom) expYearFrom.value = currentYear;
        if (expYearTo) expYearTo.value = currentYear;
        return;
    }

    // Extract years using pre-calculated value
    const years = photoData
        .map(photo => photo.year)
        .filter(year => year !== null);

    if (years.length === 0) {
        const currentYear = new Date().getFullYear();
        if (expYearFrom) expYearFrom.value = currentYear;
        if (expYearTo) expYearTo.value = currentYear;
        if (expRangeLabel) expRangeLabel.textContent = '';
        return;
    }

    const minYear = Math.min(...years);
    const maxYear = Math.max(...years);

    // Update info label
    if (expRangeLabel) {
        expRangeLabel.textContent = ` (${minYear}—${maxYear})`;
    }

    // Set initial values
    if (expYearFrom) expYearFrom.value = minYear;
    if (expYearTo) expYearTo.value = maxYear;

    // Set min/max attributes
    [expYearFrom, expYearTo].forEach(input => {
        if (input) {
            input.min = minYear;
            input.max = maxYear;
        }
    });

    // Helper to sync and filter
    function handleYearChange(source, fromInput, toInput) {
        let fromValue = parseInt(fromInput.value);
        let toValue = parseInt(toInput.value);

        // Basic bounds validation (min/max)
        if (fromValue < minYear) {
            fromValue = minYear;
            fromInput.value = minYear;
        }
        if (toValue > maxYear) {
            toValue = maxYear;
            toInput.value = maxYear;
        }

        // Validate range: From should be <= To
        // But don't auto-adjust the other field - just clamp current field
        if (source === fromInput) {
            // User changed From field
            if (fromValue > toValue) {
                fromValue = toValue;
                fromInput.value = fromValue;  // Write clamped FROM value
            }
        } else {
            // User changed To field
            if (toValue < fromValue) {
                toValue = fromValue;
                toInput.value = toValue;  // Write clamped TO value
            }
        }

        filterMarkers();
    }

    if (expYearFrom && expYearTo) {
        expYearFrom.addEventListener('input', () => handleYearChange(expYearFrom, expYearFrom, expYearTo));
        expYearTo.addEventListener('input', () => handleYearChange(expYearTo, expYearFrom, expYearTo));
    }

    console.log(`Year controls initialized: ${minYear} to ${maxYear}`);
}

// Shutdown application
async function shutdownApp() {
    if (!confirm('Are you sure you want to close PhotoMap?')) {
        return;
    }

    try {
        // 1. Get current panel position
        const panel = document.getElementById('experimental-panel');
        let top = 12;
        let left = 52;

        if (panel) {
            const rect = panel.getBoundingClientRect();
            top = Math.round(rect.top);
            left = Math.round(rect.left);
        }

        // 2. Fetch current settings to avoid overwriting other fields
        const settingsResponse = await fetch('/api/settings');
        if (settingsResponse.ok) {
            const currentSettings = await settingsResponse.json();

            // 3. Update panel position
            currentSettings.top = top;
            currentSettings.left = left;

            // 4. Update toggle states
            const mapCoordsToggle = document.getElementById('exp-map-coords-toggle');
            const routesToggle = document.getElementById('exp-routes-toggle');
            const heatmapToggle = document.getElementById('exp-heatmap-toggle');
            const browserAutostartToggle = document.getElementById('exp-browser-autostart-toggle');

            if (mapCoordsToggle) currentSettings.map_coords = mapCoordsToggle.checked;
            if (routesToggle) currentSettings.routes = routesToggle.checked;
            if (heatmapToggle) currentSettings.heatmap = heatmapToggle.checked;
            if (browserAutostartToggle) currentSettings.start_browser = browserAutostartToggle.checked;

            // 5. Save settings
            const updateResponse = await fetch('/api/update_settings', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(currentSettings)
            });

            if (updateResponse.ok) {
                showNotification('💾 Saved', 'success');
            } else {
                console.error('Failed to save settings before shutdown');
                showNotification('⚠️ Failed to save settings', 'error');
            }
        } else {
            console.error('Failed to fetch current settings');
        }

        // 5. Wait 300ms before shutdown so you can see logs
        showNotification('👋 Shutting down...', 'info');

        setTimeout(async () => {
            try {
                const response = await fetch('/api/shutdown', {
                    method: 'POST'
                });

                if (response.ok) {
                    document.body.innerHTML = '<div class="shutdown-screen"><h1>👋 PhotoMap is closed</h1><p>You can close this tab now.</p></div>';
                } else {
                    showNotification('❌ Failed to stop server', 'error');
                }
            } catch (error) {
                console.error('Shutdown error:', error);
                showNotification('❌ Error stopping server', 'error');
            }
        }, 5000);
    } catch (error) {
        console.error('Shutdown error:', error);
        showNotification('❌ Error during shutdown', 'error');
    }
}

// Load photos when page loads
loadSettings().then(() => {
    loadPhotos().then(() => {
        initializeYearControls();
        // Draw routes after settings are loaded and applied
        drawPolylines();
    });
});

// === UI Control Functions ===

// Open native folder selection dialog
// Open native folder selection dialog
async function openFolderDialog() {
    const openButton = document.getElementById('exp-open-button');
    const folderInput = document.getElementById('exp-folder-input');

    if (!openButton || !folderInput) {
        console.error('Open button or folder input not found');
        return;
    }

    const originalText = openButton.innerHTML;

    try {
        openButton.disabled = true;
        // Keep icon, change text
        openButton.innerHTML = `
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round">
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
            Wait...`;

        const response = await fetch('/api/select-folder', {
            method: 'POST'
        });

        const result = await response.json();

        if (result.status === 'success') {
            // Backend returns folder_paths array
            const folders = result.folder_paths || [];
            if (folders.length > 1) {
                folderInput.value = `Multiple folders (${folders.length})`;
                showNotification(`✅ ${folders.length} folders selected`, 'success');
            } else if (folders.length === 1) {
                folderInput.value = folders[0];
                showNotification('✅ Folder selected', 'success');
            }

            // Store folders array for processing
            window.selectedFolders = folders;

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
        if (openButton) {
            openButton.disabled = false;
            openButton.innerHTML = originalText;
        }
    }
}

// Browse for folder and immediately start processing
async function processFolder() {
    const folderInput = document.getElementById('exp-folder-input');
    const folderPath = folderInput ? folderInput.value.trim() : '';

    // Validate folder path
    if (!folderPath) {
        showNotification('❌ Please enter a folder path', 'error');
        return;
    }

    try {
        showNotification('⏳ Processing started...', 'info');

        // Step 1: Send folder paths to server (supports both single and multiple)
        // If we have selectedFolders from dialog, use that; otherwise use single path
        const foldersToSend = window.selectedFolders || [folderPath];

        const response = await fetch('/api/set-folder', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ folder_paths: foldersToSend })  // Send full array
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
            // Step 3: Reprocess (clears DB and processes folders)
            const processResponse = await fetch('/api/reprocess', {
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
                loadPhotos().then(() => {
                    initializeYearControls(); // Re-initialize year controls with new data
                }); // Refresh map
                updateStatistics();
                showNotification(`🎉 Processing completed! Found ${data.data.processed || 0} photos`, 'success');
            } else if (data.event_type === 'processing_error') {
                eventSource.close();
                showNotification(`❌ Error: ${data.data.message}`, 'error');
            } else {
                // Handle other events like progress updates
                // Optional: Show progress in notification or console
                // console.log('Processing progress:', data.data.message);
            }
        };

        eventSource.onerror = function () {
            eventSource.close();
            showNotification('❌ Error connecting to the server for updates.', 'error');
        };

    } catch (error) {
        // Handle errors
        showNotification('❌ Error: ' + error.message, 'error');
    }
}


// Show notification
// Show notification
function showNotification(message, type = 'info') {
    // Remove any existing notifications to prevent stacking
    const existingNotifications = document.querySelectorAll('.notification-toast');
    existingNotifications.forEach(n => n.remove());

    const notification = document.createElement('div');
    notification.className = `notification-toast ${type}`;

    // Add icon based on type
    let icon = '';
    if (type === 'success') icon = '✅';
    else if (type === 'error') icon = '❌';
    else icon = 'ℹ️';

    // If message already contains icon, don't add it
    if (message.includes('✅') || message.includes('❌') || message.includes('🎉') || message.includes('⏳')) {
        notification.textContent = message;
    } else {
        notification.textContent = `${icon} ${message}`;
    }

    document.body.appendChild(notification);

    setTimeout(() => {
        // Check if it's still there (might have been removed by a newer notification)
        if (document.body.contains(notification)) {
            notification.style.animation = 'slideOut 0.3s cubic-bezier(0.16, 1, 0.3, 1) forwards';
            setTimeout(() => {
                if (document.body.contains(notification)) {
                    document.body.removeChild(notification);
                }
            }, 300);
        }
    }, 3000);
}



// === Routes / Polylines Logic ===



// Event listener for toggle


// === Heatmap Logic ===

function updateHeatmap(photos) {
    if (!L.heatLayer) {
        console.warn('Heatmap plugin (leaflet-heat) is not loaded');
        return;
    }

    const points = photos.map(p => [p.lat, p.lng, 1]); // 1 is intensity

    if (heatLayer) {
        map.removeLayer(heatLayer);
    }

    try {
        heatLayer = L.heatLayer(points, {
            radius: 25,
            blur: 15,
            maxZoom: 10,
        }).addTo(map);
    } catch (error) {
        console.error('Failed to create heatmap layer:', error);
    }
}

// Event listener for heatmap toggle




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

// Calculate items per page based on screen size (both width and height)
function getItemsPerPage() {
    const width = window.innerWidth;
    const height = window.innerHeight;

    // Determine columns based on width
    let columns;
    if (width >= 1400) {
        columns = 7;
    } else if (width >= 1100) {
        columns = 6;
    } else if (width >= 900) {
        columns = 5;
    } else if (width >= 650) {
        columns = 4;
    } else if (width >= 400) {
        columns = 3;
    } else {
        columns = 2;
    }

    // Thumbnail dimensions (CSS: .cluster-thumbnail with aspect-ratio: 1)
    const THUMBNAIL_SIZE = 120;  // Base size in pixels (square)
    const THUMBNAIL_GAP = 12;    // Gap between thumbnails (CSS: gap: 12px)
    const THUMBNAIL_HEIGHT = THUMBNAIL_SIZE + THUMBNAIL_GAP;  // Total vertical space per thumbnail

    // Modal overhead (header + padding + pagination + margins)
    const MODAL_OVERHEAD = 360;  // Total vertical space used by modal chrome (tested value)

    // Determine rows based on available height
    const availableHeight = height - MODAL_OVERHEAD;
    const maxRows = Math.max(2, Math.floor(availableHeight / THUMBNAIL_HEIGHT));

    // Limit rows to reasonable range (2-6)
    const rows = Math.min(6, maxRows);

    return columns * rows;
}

// Gallery State
let galleryState = {
    photos: [],
    currentPage: 1,
    get itemsPerPage() {
        return getItemsPerPage();
    }
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
        thumb.addEventListener('click', () => showPhotoInGallery(photo));

        const img = document.createElement('img');
        img.src = `/api/gallery/${photo.relative_path}`;  // Use gallery size (240x240)
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
    detailFilename.textContent = photo.file_path;
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
function toggleExpPanel() {
    const panel = document.getElementById('experimental-panel');
    panel.classList.toggle('collapsed');
}

// Go to user's current location
function goToUserLocation() {
    if (userLocation) {
        map.setView(userLocation, 15);
        // Open popup if marker exists
        if (userLocationMarker) {
            userLocationMarker.openPopup();
        }
    } else {
        showNotification('⚠️ Location not available. Please allow geolocation.', 'warning');
        // Try to request location again
        map.locate({ setView: true, maxZoom: 15 });
    }
}

// Toggle travel routes
function toggleRoutes() {
    const toggle = document.getElementById('exp-routes-toggle');
    if (toggle && toggle.checked) {
        drawPolylines();
    } else {
        routesLayerGroup.clearLayers();
    }
}

// Draw travel routes (polylines)
function drawPolylines() {
    routesLayerGroup.clearLayers();

    // Only draw if routes toggle is checked
    const toggle = document.getElementById('exp-routes-toggle');
    if (!toggle || !toggle.checked) return;

    // Don't draw routes if heatmap is enabled (incompatible)
    const heatmapToggle = document.getElementById('exp-heatmap-toggle');
    if (heatmapToggle && heatmapToggle.checked) return;

    // Group photos by date
    const photosByDate = {};

    // Use filtered photos if available (from marker cluster), otherwise all photos
    // But we need the actual photo objects. 
    // Let's use the global photoData for now, or we could try to filter again.
    // For simplicity and performance, let's use the visible markers if possible, 
    // or just iterate photoData and check if they are in the current filter range.

    // Actually, the best way is to use the same filter logic. 
    // But let's just use photoData and filter by the current year range inputs.
    const yearFromInput = document.getElementById('exp-year-from');
    const yearToInput = document.getElementById('exp-year-to');
    const fromYear = yearFromInput ? parseInt(yearFromInput.value) : 1900;
    const toYear = yearToInput ? parseInt(yearToInput.value) : 2100;

    const activePhotos = photoData.filter(p => {
        const y = p.year;
        return y !== null && y >= fromYear && y <= toYear;
    });

    activePhotos.forEach(photo => {
        const date = photo.datetime.split(' ')[0].replace(/:/g, '-'); // YYYY-MM-DD
        if (!photosByDate[date]) {
            photosByDate[date] = [];
        }
        photosByDate[date].push(photo);
    });

    // Draw lines for each date
    Object.keys(photosByDate).forEach(date => {
        const dayPhotos = photosByDate[date];
        if (dayPhotos.length < 2) return;

        // Sort by time
        dayPhotos.sort((a, b) => a.datetime.localeCompare(b.datetime));

        const latlngs = dayPhotos.map(p => [p.lat, p.lng]);

        L.polyline(latlngs, {
            color: '#3388ff',
            weight: 3,
            opacity: 0.7,
            dashArray: '5, 10'
        }).addTo(routesLayerGroup);
    });
}

// Handle window resize for responsive gallery pagination
let resizeTimeout;
window.addEventListener('resize', () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
        // Re-render gallery if it's open and has photos
        if (galleryState.photos.length > 0) {
            const modal = document.getElementById('cluster-modal');
            if (modal && !modal.classList.contains('hidden')) {
                // Recalculate current page to avoid out-of-bounds
                const totalPages = Math.ceil(galleryState.photos.length / galleryState.itemsPerPage);
                if (galleryState.currentPage > totalPages) {
                    galleryState.currentPage = totalPages;
                }
                renderGalleryPage(galleryState.currentPage);
            }
        }
    }, 250);  // Debounce: wait 250ms after resize stops
});

// Log loaded library versions for debugging
console.log('📚 Loaded Libraries:');
console.log('  - Leaflet:', L.version || 'unknown');
console.log('  - Leaflet.markercluster:', L.MarkerClusterGroup ? '✅ loaded' : '❌ not loaded');
console.log('  - Leaflet.heat:', L.heatLayer ? '✅ loaded' : '❌ not loaded');
