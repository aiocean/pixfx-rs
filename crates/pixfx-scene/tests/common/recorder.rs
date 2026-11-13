//! A recording shell for the scene's tests (`#[path = "common/recorder.rs"] mod recorder;`).

use pixfx_draw::Frame;
use pixfx_scene::{Pass, Shell};

/// A shell that keeps each shown layer and counts what it draws.
#[derive(Default)]
pub struct Recorder {
    pub made: usize,
    pub refilled: usize,
    pub shown: Vec<(Pass, Frame)>,
}

impl Shell for Recorder {
    type Surface = Frame;
    type Error = ();
    fn make(&mut self, frame: &Frame) -> Result<Frame, ()> {
        self.made += 1;
        Ok(frame.clone())
    }
    fn refill(&mut self, surface: &mut Frame, frame: &Frame) -> Result<(), ()> {
        self.refilled += 1;
        surface.clone_from(frame);
        Ok(())
    }
    fn show(&mut self, pass: Pass, surface: &Frame) -> Result<(), ()> {
        self.shown.push((pass, surface.clone()));
        Ok(())
    }
    fn free(&mut self, _surface: Frame) -> Result<(), ()> {
        Ok(())
    }
}
