use tauri::{Runtime, Window};
use crate::{models::*, error::Error};

use crate::platform;

pub fn share_text<R: Runtime>(window: Window<R>, options: ShareTextOptions) -> Result<(), Error> {
    platform::share_text(window, options)
}

pub fn share_data<R: Runtime>(window: Window<R>, options: ShareDataOptions) -> Result<(), Error> {
    platform::share_data(window, options)
}

pub fn share_file<R: Runtime>(window: Window<R>, options: ShareFileOptions) -> Result<(), Error> {
    platform::share_file(window, options)
}

pub fn cleanup() -> Result<(), Error> {
    platform::cleanup()
}