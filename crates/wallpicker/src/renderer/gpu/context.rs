//! EGL display/context lifetime; all operations stay on the renderer thread.

use khronos_egl as egl;
use wayland_client::Connection;

pub(super) struct Context {
    pub(super) api: egl::DynamicInstance<egl::EGL1_5>,
    pub(super) display: egl::Display,
    pub(super) config: egl::Config,
    context: Option<egl::Context>,
    scratch: Option<egl::Surface>,
    _connection: Option<Connection>,
}

impl Context {
    pub(super) fn new(connection: &Connection) -> Result<Self, String> {
        // SAFETY: load the standard EGL ABI, and use this live connection's
        // wl_display with the Wayland platform (EGL_KHR_platform_wayland).
        let api = unsafe { egl::DynamicInstance::<egl::EGL1_5>::load_required() }.map_err(|e| e.to_string())?;
        let display = unsafe {
            api.get_platform_display(
                0x31D8,
                connection.backend().display_ptr().cast(),
                &[egl::NONE as egl::Attrib],
            )
        }
        .map_err(|e| e.to_string())?;
        Self::initialize(
            api,
            display,
            Some(connection.clone()),
            egl::WINDOW_BIT | egl::PBUFFER_BIT,
        )
    }

    #[cfg(test)]
    pub(super) fn headless() -> Result<Self, String> {
        // SAFETY: EGL_MESA_platform_surfaceless has no native display handle.
        let api = unsafe { egl::DynamicInstance::<egl::EGL1_5>::load_required() }.map_err(|e| e.to_string())?;
        let display = unsafe { api.get_platform_display(0x31DD, std::ptr::null_mut(), &[egl::NONE as egl::Attrib]) }
            .map_err(|e| e.to_string())?;
        Self::initialize(api, display, None, egl::PBUFFER_BIT)
    }

    fn initialize(
        api: egl::DynamicInstance<egl::EGL1_5>,
        display: egl::Display,
        connection: Option<Connection>,
        surface_type: egl::Int,
    ) -> Result<Self, String> {
        api.initialize(display).map_err(|e| e.to_string())?;
        let config = match api.choose_first_config(
            display,
            &[
                egl::SURFACE_TYPE,
                surface_type,
                egl::RENDERABLE_TYPE,
                egl::OPENGL_ES2_BIT,
                egl::RED_SIZE,
                8,
                egl::GREEN_SIZE,
                8,
                egl::BLUE_SIZE,
                8,
                egl::ALPHA_SIZE,
                8,
                egl::NONE,
            ],
        ) {
            Ok(Some(config)) => config,
            result => {
                let _ = api.terminate(display);
                return Err(format!("no RGBA EGL window config: {result:?}"));
            }
        };
        let mut this = Self {
            api,
            display,
            config,
            context: None,
            scratch: None,
            _connection: connection,
        };
        this.api.bind_api(egl::OPENGL_ES_API).map_err(|e| e.to_string())?;
        this.context = Some(
            this.api
                .create_context(display, config, None, &[egl::CONTEXT_CLIENT_VERSION, 2, egl::NONE])
                .map_err(|e| e.to_string())?,
        );
        this.scratch = Some(
            this.api
                .create_pbuffer_surface(display, config, &[egl::WIDTH, 1, egl::HEIGHT, 1, egl::NONE])
                .map_err(|e| e.to_string())?,
        );
        this.current(None)?;
        Ok(this)
    }

    pub(super) fn current(&self, surface: Option<egl::Surface>) -> Result<(), String> {
        let surface = surface.or(self.scratch);
        self.api
            .make_current(self.display, surface, surface, self.context)
            .map_err(|e| e.to_string())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        let _ = self.api.make_current(self.display, None, None, None);
        if let Some(surface) = self.scratch.take() {
            let _ = self.api.destroy_surface(self.display, surface);
        }
        if let Some(context) = self.context.take() {
            let _ = self.api.destroy_context(self.display, context);
        }
        let _ = self.api.terminate(self.display);
    }
}
