use std::sync::Arc;
use winit::window::Window;

use crate::render::Renderer;

pub(crate) struct State {
    renderer: Renderer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let renderer = Renderer::new(window).await;

        State { renderer }
    }

    pub fn get_window(&self) -> &Window {
        self.renderer.window()
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    pub fn render(&mut self) {
        self.renderer.render();
    }
}
