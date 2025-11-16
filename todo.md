# PhotoMap v3.1 - UI Improvement Plan

## 🎯 Goal
Unified user interface with web-based folder selection and real-time progress tracking

## 📋 Current Issues
- Split interface: console + web map
- Hardcoded photos directory
- No folder selection in UI
- Console messages in Russian (need English)
- Port conflicts not handled
- No progress visualization during processing

## 🎨 New Layout Design
```
┌─────────────────────────────────┬─────────────────────────┐
│                                 │    📊 PhotoMap Status    │
│         🗺️  Map Area            │                         │
│         (80% width)             │    📂 Folder: [Choose...] │
│                                 │                         │
│                                 │    🔄 [Process] Button    │
│                                 │─────────────────────────│
│                                 │    ⏳ Processing status   │
│                                 │    📊 Progress info       │
│                                 │    📊 Statistics         │
│                                 │                         │
│                                 │    🎉 Results            │
└─────────────────────────────────┴─────────────────────────┘
```

## 📋 Implementation Plan

### Phase 1: Layout & UI Structure ✅ COMPLETED
- [x] Create two-column layout (80% map, 25% info panel)
- [x] Responsive design with CSS Grid
- [x] Info panel full height on right side
- [x] Status section at top of panel
- [x] Progress section in middle
- [x] Results section at bottom
- [x] Map statistics with dynamic sector counting

### Phase 2: Folder Selection & Settings 🔄 IN PROGRESS
- [x] Add rfd crate dependency
- [x] Create settings.rs module for INI file
- [x] INI file persistence in project directory
- [x] Show selected folder path in info panel
- [x] "Обзор" button for folder selection (placeholder)
- [ ] **Native folder dialog integration** (CURRENT TASK)
- [ ] Remember last folder in INI file
- [ ] Enable "Process" button when folder selected
- [ ] Connect folder selection to actual processing

### Phase 3: Real-time Updates ✅ COMPLETED
- [x] Add SSE (Server-Sent Events) support
- [x] Create `/api/events` endpoint
- [x] Real-time progress updates in info panel
- [x] Processing speed indicator
- [x] Connect SSE to actual photo processing
- [x] `/api/process` endpoint for triggering processing

### Phase 4: Port Management ✅ COMPLETED
- [x] Check if port 3001 is available
- [x] Kill existing process if port occupied
- [x] Cross-platform process killing (Windows/macOS/Linux)
- [x] Fallback ports (3001 → 3002 → 3003)
- [x] Show port status in console

### Phase 5: Localization & UI Improvements ✅ COMPLETED
- [x] Move server info lines from HTML to console
- [x] Update version to v0.3.1
- [x] Console messages for user guidance
- [x] Web interface in English
- [x] Keep Russian communication with user

### Phase 6: Image Processing Optimizations ✅ COMPLETED
- [x] Optimize image sizes: 60px thumbnails, 40px markers
- [x] Fix HEIC thumbnail aspect ratio issues
- [x] Ensure consistent behavior between JPEG and HEIC
- [x] Update server documentation for new sizes

## 🛠️ Technical Requirements

### New Dependencies
```toml
rfd = "0.12"                    # Native folder dialog
serde_ini = "0.2"               # INI file support
sysinfo = "0.30"                 # Process management
tokio-stream = "0.1"            # For SSE
futures-util = "0.3"            # Stream utilities
```

### New Modules
- `settings.rs` - INI file management
- `folder_picker.rs` - rfd integration
- `port_manager.rs` - Cross-platform port management
- `events.rs` - SSE event handling

### State Management
```rust
enum UIState {
    WaitingForFolder {
        last_folder: Option<String>,
    },
    ReadyToProcess {
        folder_path: String,
        stats: FolderStats,
    },
    Processing {
        folder_path: String,
        current: usize,
        total: usize,
        stats: ProcessingStats,
    },
    Completed {
        folder_path: String,
        final_stats: FinalStats,
    },
    Error {
        message: String,
    }
}
```

### INI File Structure
```ini
[PhotoMap]
last_folder = /Users/name/Pictures/Vacation2024
port = 3001
auto_open_browser = false

[Display]
info_panel_width = 20
show_progress = true
```

## 🎯 User Flow

### 1. Application Start
```bash
./photomap_processor
```
Console output:
```
🗺️  PhotoMap Processor v3.1
🌐 Opening http://127.0.0.1:3001...
⚡ Server ready on port 3001
```

### 2. Web Interface
- Load last folder from INI file
- Show "Last used: /path/to/folder" or "Choose folder"
- "Process" button disabled until folder selected
- Info panel shows "Waiting for folder selection"

### 3. Folder Selection
- User clicks "Choose Folder" → native dialog opens
- Selected folder path displayed
- "Process" button becomes enabled
- Show folder statistics if available

### 4. Processing
- Real-time updates in info panel
- Progress bar and statistics
- Processing speed and ETA
- Map updates automatically when complete

### 5. Results
- Show final statistics
- Options to process new folder or reprocess current
- Auto-reload map if requested

## 🔧 Implementation Details

### Info Panel Sections
1. **Folder Selection**
   - Last used folder or choose new
   - Process button

2. **Processing Status**
   - Current operation (Scanning/Processing)
   - Progress indicator
   - Speed and ETA

3. **Statistics**
   - Total files found
   - Photos with GPS
   - Photos without GPS
   - HEIC files count
   - Skipped files

4. **Results**
   - Final statistics
   - Processing time
   - Action buttons

### SSE Events
```json
{
  "type": "processing_start",
  "folder": "/path/to/folder",
  "total_files": 10000
}

{
  "type": "progress",
  "current": 5000,
  "total": 10000,
  "gps_found": 2000,
  "no_gps": 3000,
  "speed": 150
}

{
  "type": "completed",
  "final_stats": {
    "total": 10000,
    "on_map": 2000,
    "no_gps": 8000
  }
}
```

## 🚨 Error Handling
- Console: Detailed error messages for debugging
- Web: Basic error messages for users
- Automatic recovery where possible
- Clear error states in info panel

## 📱 Cross-platform Requirements
- Windows: Native folder dialog + process killing
- macOS: Native folder dialog + process killing
- Linux: Native folder dialog + process killing
- Browser: Modern browsers with SSE support

## ✅ Success Criteria
- Single unified web interface
- Cross-platform folder selection
- Real-time progress tracking
- Settings persistence
- Automatic port management
- All interface elements in English
- Robust error handling

## 🎯 Next Steps
1. **CURRENT: Native folder dialog integration with rfd crate**
2. Connect folder selection to actual photo processing
3. Remember last folder in INI file
4. Enable/disable "Process" button based on folder selection
5. Testing and refinement of complete workflow
6. Performance optimization for large photo collections (10,000+)