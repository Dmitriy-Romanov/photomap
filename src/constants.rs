// Port configuration
pub const DEFAULT_PORT: u16 = 3001;
pub const ALTERNATIVE_PORT: u16 = 3002;
pub const PORTS_TO_CHECK: &[u16] = &[3001, 3002];

// Image sizes
pub const MARKER_SIZE: u32 = 40;
pub const THUMBNAIL_SIZE: u32 = 60;

// ImageMagick parameters - CRITICAL: These parameters preserve aspect ratio!
// FORBIDDEN: Never use ^ (force) parameters as they make HEIC thumbnails square
// ALLOWED: Only use > (only if larger) parameters to preserve aspect ratio
pub const MARKER_RESIZE_PARAMS: &str = "40x40>";  // Only resize if larger, preserve aspect ratio
pub const THUMBNAIL_RESIZE_PARAMS: &str = "60x60>";  // Only resize if larger, preserve aspect ratio