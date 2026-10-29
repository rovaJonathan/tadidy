//! `NSPasteboard.generalPasteboard` behind the [`Pasteboard`] trait. Needs the real
//! pasteboard, so it is covered by the manual QA of TAD-004, not by a unit test; the contract
//! it honours is pinned by the tests of `crate::platform::fakes`.

use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem};
use objc2_foundation::{NSArray, NSData, NSString};

use crate::platform::{Pasteboard, PlatformError, Representation};

/// The general pasteboard. Stateless: every call asks AppKit again.
#[derive(Debug, Default, Clone, Copy)]
pub struct NsPasteboard;

/// The first item only: multi-item copies (several files) belong to the files ticket.
fn first_item(pasteboard: &NSPasteboard) -> Option<Retained<NSPasteboardItem>> {
    pasteboard.pasteboardItems()?.firstObject()
}

impl Pasteboard for NsPasteboard {
    fn change_count(&self) -> i64 {
        autoreleasepool(|_| NSPasteboard::generalPasteboard().changeCount() as i64)
    }

    fn types(&self) -> Vec<String> {
        autoreleasepool(|_| {
            first_item(&NSPasteboard::generalPasteboard())
                .map(|item| item.types().iter().map(|uti| uti.to_string()).collect())
                .unwrap_or_default()
        })
    }

    fn read(&self, uti: &str) -> Option<Vec<u8>> {
        autoreleasepool(|_| {
            let item = first_item(&NSPasteboard::generalPasteboard())?;
            let data = item.dataForType(&NSString::from_str(uti))?;
            Some(data.to_vec())
        })
    }

    fn write(&self, items: &[Representation]) -> Result<i64, PlatformError> {
        autoreleasepool(|_| {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            let item = NSPasteboardItem::new();
            for representation in items {
                let data = NSData::with_bytes(&representation.data);
                let uti = NSString::from_str(&representation.uti);
                if !item.setData_forType(&data, &uti) {
                    return Err(PlatformError::PasteboardWrite);
                }
            }
            let objects = NSArray::from_retained_slice(&[ProtocolObject::from_retained(item)]);
            if !pasteboard.writeObjects(&objects) {
                return Err(PlatformError::PasteboardWrite);
            }
            Ok(pasteboard.changeCount() as i64)
        })
    }
}
