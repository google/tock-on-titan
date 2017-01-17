use hotel::hil::rng::{Continue, Rng, RngClient};
use kernel::{AppId, AppSlice, Callback, Container, Driver, Shared};
use kernel::common::take_cell::TakeCell;

pub struct App {
    callback: Option<Callback>,
    buffer: Option<AppSlice<Shared, u8>>,
    offset: usize,
}

impl Default for App {
    fn default() -> App {
        App {
            callback: None,
            buffer: None,
            offset: usize::max_value(),
        }
    }
}

/// Driver for a random number generator, using the Rng trait.
pub struct RngDriver<'a, G: Rng + 'a> {
    rng: TakeCell<&'a mut G>,
    apps: Container<App>,
}

impl<'a, G: Rng + 'a> RngDriver<'a, G> {
    /// Creates a new RngDriver.
    pub fn new(rng: &'a mut G, container: Container<App>) -> RngDriver<'a, G> {
        RngDriver {
            rng: TakeCell::new(rng),
            apps: container,
        }
    }
}

impl<'a, G: Rng + 'a> Driver for RngDriver<'a, G> {
    /// Saves an application-provided buffer to be filled with random data.
    fn allow(&self, app_id: AppId, _: usize, slice: AppSlice<Shared, u8>) -> isize {
        self.apps
            .enter(app_id, |app, _| {
                app.buffer = Some(slice);
                app.offset = 0;
                0
            })
            .unwrap_or(-1)
    }

    /// Saves an application-provided callback that will be used to notify
    /// the application when the provided buffer is full.
    fn subscribe(&self, _: usize, callback: Callback) -> isize {
        self.apps
            .enter(callback.app_id(), |app, _| {
                app.callback = Some(callback);
                0
            })
            .unwrap_or(-1)
    }

    /// Instructs the driver to begin filling the application-provided buffer with
    /// random data.  If the application has not provided both a buffer to fill and
    /// a notification callback this will return an error.
    fn command(&self, _: usize, _: usize, app_id: AppId) -> isize {
        self.apps
            .enter(app_id, |app, _| {
                if app.callback.is_none() || app.buffer.is_none() {
                    return -1;
                }

                self.rng
                    .map(|rng| {
                        rng.get_data();
                        0
                    })
                    .unwrap_or(-1)
            })
            .unwrap_or(-1)
    }
}

impl<'a, G: Rng + 'a> RngClient for RngDriver<'a, G> {
    fn random_data_available(&self, iter: &mut Iterator<Item = u32>) -> Continue {
        for container in self.apps.iter() {
            let finished = container.enter(|app, _| {
                if app.callback.is_none() || app.buffer.is_none() {
                    // These may not be fully set up yet.
                    return true;
                }

                // Take the buffer out.
                let mut slice = app.buffer.take().unwrap();
                {
                    // Fill the buffer with random data.
                    let buf: &mut [u8] = slice.as_mut();
                    while let Some(data) = iter.next() {
                        if app.offset >= buf.len() {
                            break;
                        }

                        let diff = buf.len() - app.offset;
                        let data = u32_to_byte_array(data);
                        if diff > 4 {
                            buf[app.offset..app.offset + 4].copy_from_slice(&data);
                            app.offset += 4;
                        } else {
                            buf[app.offset..].copy_from_slice(&data[..diff]);
                            app.offset += diff;
                        }
                    }
                }
                // Put the buffer back.
                app.buffer = Some(slice);

                if app.offset < app.buffer.as_ref().unwrap().len() {
                    return false;
                }

                // The buffer is full.  Notify the application.
                app.callback.map(|mut cb| cb.schedule(app.offset, 0, 0));

                // Reset the Container
                app.callback = None;
                app.buffer = None;
                app.offset = usize::max_value();

                true
            });

            if !finished {
                return Continue::More;
            }
        }

        Continue::Done
    }
}

fn u32_to_byte_array(x: u32) -> [u8; 4] {
    let x1 = (x & 0xff) as u8;
    let x2 = ((x >> 8) & 0xff) as u8;
    let x3 = ((x >> 16) & 0xff) as u8;
    let x4 = ((x >> 24) & 0xff) as u8;

    [x1, x2, x3, x4]
}
