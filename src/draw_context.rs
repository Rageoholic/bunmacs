use std::sync::Arc;

use wgpu::Instance;
use winit::window::{Window, WindowId};

//TODO: Genericize over backend?

pub(crate) struct WgpuInfo {
    _shared_context: Arc<SharedWgpuContext>,
}

struct SharedWgpuContext {
    _instance: Instance,
}

impl WgpuInfo {
    pub(crate) fn new(initial_window: Window) -> (Self, WindowContext) {
        let instance = Instance::new(Default::default());
        let inner = Arc::new(SharedWgpuContext {
            _instance: instance,
        });
        (
            WgpuInfo {
                _shared_context: inner.clone(),
            },
            WindowContext {
                win: initial_window,
                _wgpu_info: inner,
            },
        )
    }
}

pub(crate) struct WindowContext {
    win: Window,
    _wgpu_info: Arc<SharedWgpuContext>,
}

impl WindowContext {
    pub fn id(&self) -> WindowId {
        self.win.id()
    }

    pub fn set_visible(&self, b: bool) {
        self.win.set_visible(b)
    }
}
