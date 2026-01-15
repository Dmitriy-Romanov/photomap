# PhotoMap Roadmap

## 🎯 Development Philosophy

- **Simplicity First**: Solve problems directly with minimal dependencies
- **Standard Library Priority**: Use std:: before external crates
- **Cross-Platform First**: Windows/macOS/Linux support is mandatory
- **Research-Driven**: Best practices research for each feature
- **User-Focused**: Features are implemented based on user needs, not trends

## 📍 Current Status: v0.9.6 (Stable)

The project has reached a high level of stability and feature completeness. The core functionality—processing photos, extracting metadata (including offline reverse geocoding), and displaying them on a high-performance map—is working robustly on macOS, Windows, and Linux.

### ✅ Recently Completed Milestones

- **Offline Reverse Geocoding**: Embedded 140k+ cities database (GeoNames) with KD-Tree for instant, offline location naming.
- **Multi-Folder Support**: Capability to process up to 5 distinct folders simultaneously.
- **Unified UI**: Consistent styling across map markers, popups, and the gallery view.
- **Performance**: Lazy initialization of heavy modules, optimized startup time.
- **Cross-Platform**: Full Windows support including proper path handling and Explorer integration.

## 📅 Future Plans & Maintenance

We are currently in a **Maintenance & Polish** phase. No major feature overhauls are planned unless driven by specific user needs.

### v1.0.0 - The "Gold" Release (Candidate)

Target: Final polish and stability verification for a major version number.

**Potential Tasks:**
- [ ] **Dependency Audit**: Review and update crates to their latest stable versions.
- [ ] **Code Cleanup**: Remove any lingering debug code or unused modules.
- [ ] **Final UI Polish**: Ensure perfect alignment and consistenty across all operating systems.
- [ ] **Documentation**: Ensure all docs are perfectly synced with behavior.

### 🔮 Backlog / Ideas (Low Priority)

These are ideas that have been discussed but are not currently scheduled:

- **Advanced Search**: Searching photos by city name (now that we have geocoding).
- **Date/Time Timeline**: A visual timeline slider for navigating years/months.
- **Cloud Integration**: (Unlikely, as local-first is a core value).
- **AI Categorization**: Using local models to categorize photos (e.g., "Nature", "Urban").

## 🔧 Technical Debt & Improvements

- **Monitoring**: Keep an eye on `chrono`, `tokio`, and `image` crate updates for performance wins.
- **Refactoring**: Continue distinguishing between "scripting" logic and "application" logic if the codebase grows.

## 🔄 Release Process

1. **Development**: Implement changes on `main` branch.
2. **Testing**:
   - Verify on macOS (Primary dev environment)
   - Verify on Windows (Excellent support, crucial for cross-platform promise)
   - Verify on Linux (Ubuntu) (Confirmed working)
3. **Release**:
   - Update `Cargo.toml` version.
   - Update `CHANGELOG.md`.
   - Update `README.md` if user-facing features changed.
   - Tag commit.

---

*This roadmap is a living document. Last updated: January 2026.*