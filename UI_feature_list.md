# UI & Leaflet Feature Suggestions

This document tracks ideas for improving the PhotoMap user interface and map interactions.

## Implemented ✅
- [x] **Browser Autostart Toggle**: Added a toggle in the info panel to control automatic browser opening.
- [x] **UI Polish**: Enforced "Sentence case" for labels and added a GitHub link to the title.

## Planned / Under Consideration 📝

### Map Interactions
1.  **Dynamic Polylines**:
    -   Draw lines between photos based on time (chronological order).
    -   Color-code lines based on time gaps (e.g., different colors for different days).
    -   Add arrows to indicate direction of travel.

2.  **Cluster Improvements**:
    -   **Spiderfy**: When clicking a cluster at max zoom, "spiderfy" the markers (spread them out in a spiral/circle) so individual photos can be selected.
    -   **Hover Previews**: Show a small thumbnail grid when hovering over a cluster icon.

3.  **Photo Filtering**:
    -   **Date Range Slider**: Replace or augment the year input fields with a visual slider.
    -   **Tag Filtering**: Filter photos by tags (if implemented in metadata).

4.  **Visual Enhancements**:
    -   **Custom Map Tiles**: Allow users to switch between different map providers (OpenStreetMap, Satellite, Dark Mode).
    -   **Marker Animations**: Add subtle bounce or fade-in animations when markers appear.

5.  **Info Panel**:
    -   **Collapsible Sections**: Allow users to collapse the "Year range" or "Source folder" sections to save space.
    -   **Draggable Panel**: Make the floating info window draggable.
