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
    <title>PhotoMap v3.0 - SQLite + Clustering + On-demand</title>
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
    <!-- HEIC_WARNING_PLACEHOLDER -->
    <div id="map"></div>

    <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
    <script src="https://unpkg.com/leaflet.markercluster@1.5.3/dist/leaflet.markercluster.js"></script>
    <script>
        // Initialize map
        const map = L.map('map').setView([52.5, 13.4], 10);

        // Add tile layer
        L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
            attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
        }).addTo(map);

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
                iconUrl: apiUrl + '/' + photo.filename,
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
                        <div class="filename">${photo.filename}</div>
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

            // Update info
            const info = L.control();
            info.onAdd = function (map) {
                this._div = L.DomUtil.create('div', 'info');
                this.update();
                return this._div;
            };
            info.update = function (state) {
                const bounds = map.getBounds();
                const visiblePhotos = photoData.filter(photo =>
                    bounds.contains([photo.lat, photo.lng])
                );

                this._div.innerHTML = '<h4>🗺️ PhotoMap v3.0</h4>' +
                    '<b>SQLite + Clustering + On-demand</b><br />' +
                    `Всего фото с GPS: ${photoData.length}<br/>` +
                    `В текущем секторе: ${visiblePhotos.length}<br/>` +
                    `<small>Путь к фото: /Users/dmitriiromanov/claude/photomap/photos</small><br/>` +
                    `<small>Кластеры: автоматическая группировка</small>`;
            };
            info.addTo(map);

            // Update info when map moves or zooms
            map.on('moveend zoomend', function() {
                info.update();
            });

            // Add zoom controls info
            map.on('zoomend', function() {
                const zoom = map.getZoom();
                console.log('Current zoom level:', zoom);
            });
        }

        // Load photos when page loads
        loadPhotos();
    </script>
</body>
</html>"#;