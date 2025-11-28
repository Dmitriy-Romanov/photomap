# UI & Map Features

This document documents the implemented user interface features and map interactions in PhotoMap v0.9.5.

## 🗺️ Main Map Interface

*   **Interactive Map**: Full-screen Leaflet.js map with smooth zooming and panning.
*   **Marker Clustering**: Automatically groups nearby photos into clusters.
    *   **Spiderfy**: Clicking small clusters expands them to show individual markers.
    *   **Cluster Gallery**: Clicking large clusters opens a modal gallery view.
*   **Photo Popups**:
    *   **Thumbnail**: High-quality, square-cropped thumbnail (240x240px).
    *   **Metadata**: Displays filename and capture date.
    *   **"Reveal File"**: Clicking the filename reveals the original file in the OS file explorer (Windows/macOS).
*   **User Location**:
    *   **Green Marker**: Shows the user's current geolocation.
    *   **"Where I" Button**: Instantly centers the map on the user's location.
*   **Crosshair**: Optional center crosshair for precise navigation.

## 🎛️ Control Panel ("Experimental Panel")

A modern, floating, draggable control panel containing all application settings.

### 1. Header & Window Controls
*   **Draggable**: Click and drag anywhere on the panel to reposition it.
*   **Coordinates**: Real-time display of map center latitude/longitude.
*   **Window Controls**:
    *   **Minimize**: Collapses the panel to a compact header bar.
    *   **Close**: Gracefully shuts down the application and closes the browser tab.

### 2. Visualization Toggles
*   **Map Coord**: Toggles the coordinate display and center crosshair.
*   **Routes**: Toggles polyline connections between photos taken on the same day (visualizes travel paths).
*   **Heatmap**: Toggles a heatmap layer showing photo density (hotspots).
*   **Browser Autostart**: Controls whether the browser launches automatically on app start.

### 3. Folder Management
*   **Native Dialog**: "Open" button triggers the OS-native folder picker (supports multi-selection).
*   **Path Display**: Shows the currently selected source folder(s).
*   **Process Button**: Manually triggers a re-scan of the selected folder.

### 4. Filters & Statistics
*   **Year Range Filter**:
    *   **From/To Inputs**: Filter displayed markers by capture year.
    *   **Dynamic Labels**: Shows the absolute min/max years available in the database.
*   **Live Statistics**:
    *   **Total Photos**: Count of all photos in the database.
    *   **Displayed**: Count of photos currently visible on the map (matching filters).

## 🖼️ Cluster Gallery

A specialized modal interface for viewing dense clusters of photos.

*   **Grid View**: Displays thumbnails of all photos in the cluster.
*   **Pagination**: Handles large clusters efficiently with page-based navigation.
*   **Detail View**: Clicking a thumbnail shows the full-size image with metadata.
*   **Keyboard Navigation**: Support for `Esc` to close or go back.

## 🎨 Visual Polish

*   **SVG Icons**: Crisp, scalable icons for all buttons using an SVG sprite system.
*   **Glassmorphism**: Translucent panel background with blur effect.
*   **Responsive Layout**: Row-based flexbox layout that adapts to content.
*   **Toast Notifications**: Non-intrusive popup notifications for actions (e.g., "Folder set", "Settings saved").
