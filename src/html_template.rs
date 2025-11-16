use axum::response::Html;

pub fn get_map_html(has_heic_support: bool) -> Html<String> {
    let heic_warning = if !has_heic_support {
        r#"<div style="background-color: #ff6b6b; color: white; padding: 8px; text-align: center; font-weight: bold; margin-bottom: 5px;">
            ⚠️ ImageMagick не установлен - HEIC файлы не обрабатываются
            <br><small>Установите: brew install imagemagick</small>
        </div>"#
    } else {
        ""
    };

    let html = MAP_HTML.replace("<!-- HEIC_WARNING_PLACEHOLDER -->", heic_warning);
    Html(html)
}

// HTML template for the map page
const MAP_HTML: &str = r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PhotoMap v0.4.1 - Native Browser Folder Selection</title>
    <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
    <link rel="stylesheet" href="https://unpkg.com/leaflet.markercluster@1.5.3/dist/MarkerCluster.css" />
    <link rel="stylesheet" href="https://unpkg.com/leaflet.markercluster@1.5.3/dist/MarkerCluster.Default.css" />
    <style>
        body { margin: 0; padding: 0; font-family: Arial, sans-serif; }
        #map { height: 100vh; width: 100%; }
        .info {
            padding: 6px 8px;
            font: 14px/16px Arial, Helvetica, sans-serif;
            background: white;
            background: rgba(255,255,255,0.9);
            box-shadow: 0 0 15px rgba(0,0,0,0.2);
            border-radius: 5px;
        }
        .info h4 {
            margin: 0 0 5px;
            color: #777;
        }
        .photo-popup {
            text-align: center;
            min-width: 720px; /* Ensure minimum width for 700px images */
            max-width: 720px; /* Fixed width for consistent layout */
        }
        .leaflet-popup-content-wrapper {
            min-width: 740px !important; /* Popup wrapper with padding */
        }
        .leaflet-popup-content {
            min-width: 720px !important;
            max-width: 720px !important;
        }
        .photo-popup img {
            max-width: 700px;
            max-height: 500px;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.3);
            margin-bottom: 8px;
        }
        .photo-popup .filename {
            font-weight: bold;
            margin: 4px 0;
            font-size: 0.9em;
        }
        .photo-popup .datetime {
            color: #666;
            margin: 4px 0;
            font-size: 0.8em;
        }
        .thumbnail-icon {
            border-radius: 4px;
            border: 2px solid white;
            box-shadow: 0 1px 3px rgba(0,0,0,0.3);
        }
        .custom-cluster-icon {
            background: #4285f4;
            border-radius: 50%;
            color: white;
            text-align: center;
            font-weight: bold;
            font-size: 12px;
            border: 2px solid white;
            box-shadow: 0 1px 3px rgba(0,0,0,0.3);
        }

            </style>
</head>
<body>
    <div style="display: flex; height: 100vh; margin: 0; padding: 0;">
        <!-- Left frame - Map -->
        <div id="map" style="flex: 1; height: 100%;"></div>

        <!-- Right frame - Info panel -->
        <div id="info-panel" style="width: 25%; min-width: 400px; height: 100vh; background: white; border-left: 2px solid #ccc; overflow-y: auto;">
            <!-- HEIC_WARNING_PLACEHOLDER -->
            <!-- Control Panel -->
            <div id="control-panel" style="position: relative; top: 10px; left: 10px; right: 10px; z-index: 1000; background: white; padding: 15px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.2);">
                <h3 style="margin: 0 0 10px 0; color: #333;">🗺️ PhotoMap v0.4.1</h3>

                <!-- Folder Selection -->
                <div style="margin-bottom: 10px;">
                    <input type="text" id="folder-input" style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 4px;" readonly placeholder="Выберите папку...">
                    <input type="file" id="folder-input-hidden" style="display: none;" webkitdirectory directory multiple>
                </div>

                <!-- Processing Controls -->
                <div style="margin-bottom: 10px;">
                    <button id="browse-button" onclick="browseAndProcessFolder()" style="background: #007bff; color: white; border: none; padding: 10px 15px; border-radius: 4px; cursor: pointer; margin-right: 10px;">
                        📁 Обзор
                    </button>
                </div>

                <!-- Processing Status -->
                <div id="processing-status" style="margin-bottom: 10px; padding: 10px; background: #f8f9fa; border-radius: 4px; display: none;">
                    <div id="status-text" style="font-weight: bold;">Обработка...</div>
                    <div id="progress-text" style="font-size: 0.9em; color: #666;"></div>
                </div>

                <!-- Statistics Display -->
                <div id="statistics-panel" style="margin-bottom: 10px; padding: 12px; background: #f0f8ff; border-radius: 6px; border: 1px solid #ddd;">
                    <h4 style="margin: 0 0 8px 0; color: #333; font-size: 1.0em;">📊 Статистика</h4>
                    <div style="font-size: 0.9em; line-height: 1.6;">
                        <div style="margin-bottom: 4px;"><strong>Всего фото:</strong> <span id="total-photos">-</span></div>
                        <div style="margin-bottom: 4px;"><strong>Отображено:</strong> <span id="visible-photos">-</span></div>
                    </div>
                </div>

              </div>
        </div>
    </div>

    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <script src="https://unpkg.com/leaflet.markercluster@1.5.3/dist/leaflet.markercluster.js"></script>
    <script>
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
        let photoData = [];

        async function loadPhotos() {
            try {
                // Clear existing markers before loading new ones
                markerClusterGroup.clearLayers();
                photoData = [];

                const response = await fetch('/api/photos');
                photoData = await response.json();
                console.log(`Loaded ${photoData.length} photos from database`);
                addMarkers();
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
                    document.getElementById('folder-input').value = 'Выберите папку...';
                }
            } catch (error) {
                console.error('Failed to load settings:', error);
                document.getElementById('folder-input').value = 'Выберите папку...';
            }
        }

        // Load photos when page loads
        loadSettings();
        loadPhotos();

        // === UI Control Functions ===

        // Browse for folder and immediately start processing
        async function browseAndProcessFolder() {
            const browseButton = document.getElementById('browse-button');
            const folderInput = document.getElementById('folder-input');
            const statusDiv = document.getElementById('processing-status');
            const statusText = document.getElementById('status-text');
            const progressText = document.getElementById('progress-text');

            try {
                // Step 1: Select folder using native browser dialog
                browseButton.disabled = true;
                browseButton.textContent = '📂 Выбор папки...';
                folderInput.value = 'Выбор папки...';

                // Create a promise to handle folder selection
                const folderSelection = new Promise((resolve, reject) => {
                    const hiddenInput = document.getElementById('folder-input-hidden');

                    hiddenInput.onchange = function(e) {
                        const files = e.target.files;
                        if (files && files.length > 0) {
                            // Get the folder path from the first file
                            const firstFile = files[0];
                            const fullPath = firstFile.webkitRelativePath;
                            const folderPath = fullPath.split('/')[0];
                            resolve(folderPath);
                        } else {
                            reject(new Error('Folder selection cancelled'));
                        }
                        // Reset the input so we can select the same folder again
                        hiddenInput.value = '';
                    };

                    hiddenInput.click();
                });

                // Wait for folder selection
                const folderPath = await folderSelection;

                showNotification(`✅ Папка выбрана: ${folderPath}`, 'success');

                // Step 2: Send folder path to server
                browseButton.textContent = '📂 Установка папки...';

                const response = await fetch('/api/set-folder', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ folder_path: folderPath })
                });

                const result = await response.json();

                if (result.status !== 'success') {
                    throw new Error(result.message || 'Ошибка установки папки');
                }

                // Update folder input with the path from server response
                folderInput.value = result.folder_path;

                // Step 3: Start processing
                browseButton.textContent = '⏳ Обработка...';
                statusDiv.style.display = 'block';
                statusText.textContent = 'Обработка фотографий...';
                progressText.textContent = 'Анализ папки...';

                const processResponse = await fetch('/api/process', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    }
                });

                const processResult = await processResponse.json();

                if (processResult.status === 'started') {
                    showNotification('✅ Обработка запущена: ' + folderPath, 'success');

                    // Check for completion periodically
                    const checkCompletion = setInterval(async () => {
                        try {
                            const photosResponse = await fetch('/api/photos');
                            const photos = await photosResponse.json();

                            if (photos.length > 0) {
                                clearInterval(checkCompletion);
                                statusDiv.style.display = 'none';
                                browseButton.disabled = false;
                                browseButton.textContent = '📁 Обзор';
                                loadPhotos(); // Refresh map
                                updateStatistics();
                                showNotification(`🎉 Обработка завершена! Найдено ${photos.length} фотографий`, 'success');
                            }
                        } catch (error) {
                            console.error('Error checking completion:', error);
                        }
                    }, 1000);
                } else {
                    throw new Error(processResult.message || 'Ошибка запуска обработки');
                }

            } catch (error) {
                // Handle errors (including cancellation)
                statusDiv.style.display = 'none';
                browseButton.disabled = false;
                browseButton.textContent = '📁 Обзор';

                if (error.message === 'Folder selection cancelled') {
                    folderInput.value = 'Папка не выбрана';
                    showNotification('🚫 Выбор папки отменен', 'info');
                } else {
                    folderInput.value = 'Ошибка выбора папки';
                    showNotification('❌ Ошибка: ' + error.message, 'error');
                }
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

        // Add CSS animations
        const style = document.createElement('style');
        style.textContent = `
            @keyframes slideIn {
                from { transform: translateX(100%); opacity: 0; }
                to { transform: translateX(0); opacity: 1; }
            }
            @keyframes slideOut {
                from { transform: translateX(0); opacity: 1; }
                to { transform: translateX(100%); opacity: 0; }
            }
        `;
        document.head.appendChild(style);

  
    </script>
</body>
</html>"#;