//! Renders a running application's icon as a PNG. Needs AppKit drawing, so it is covered by
//! the manual QA of TAD-004, not by a unit test.

use std::ptr;

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSDeviceRGBColorSpace,
    NSGraphicsContext, NSRunningApplication,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize};

/// Side of the PNG in pixels.
const PIXELS: isize = 64;
/// The same side in points: the PNG is a 32 pt icon @2x.
const POINTS: f64 = 32.0;

/// `app`'s icon as a 64 × 64 px PNG, `None` when AppKit cannot draw or encode it.
/// The caller holds an autorelease pool.
pub fn png(app: &NSRunningApplication) -> Option<Vec<u8>> {
    let image = app.icon()?;
    let bitmap = new_bitmap()?;
    let context = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;

    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&context));
    image.drawInRect_fromRect_operation_fraction(
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(POINTS, POINTS)),
        NSRect::ZERO, // the whole image
        NSCompositingOperation::SourceOver,
        1.0,
    );
    NSGraphicsContext::restoreGraphicsState_class();

    // SAFETY: the properties dictionary is empty, so no value has a wrong type.
    let data = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }?;
    Some(data.to_vec())
}

/// An empty 64 × 64 px RGBA bitmap whose size in points is 32 × 32 (so it draws @2x).
fn new_bitmap() -> Option<Retained<NSBitmapImageRep>> {
    // SAFETY: a null `planes` asks AppKit to allocate the pixel buffer; the other arguments
    // describe a plain 8-bit RGBA bitmap, with 0 for "compute the row size and bit depth".
    let bitmap = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            ptr::null_mut(),
            PIXELS,
            PIXELS,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            0,
            0,
        )
    }?;
    bitmap.setSize(NSSize::new(POINTS, POINTS));
    Some(bitmap)
}
