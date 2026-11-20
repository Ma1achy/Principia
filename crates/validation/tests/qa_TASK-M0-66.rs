//! QA's tests for TASK-M0-66, written from REQ-TOOL-238: "the fast-math checks must read the API from the opened
//! adapter's backend, not from the session header, which refuses a discrete GPU". The header's refusal is unchanged
//! (the task's P1), so for an adapter of each kind on each backend the header records, `header_api` of its backend
//! names the API the header would record for it, the discrete adapter's included, which the header itself refuses.
//! Its negative control (R-176) is the reading the fast-math checks made before: the API through the header, which
//! loses a discrete GPU's.

use engine::contract::profile::Api;
use validation::gpu::{header_api, session_adapter};
use validation::negative_control;

const RECORDED: [(wgpu::Backend, Api); 4] = [
    (wgpu::Backend::Metal, Api::Metal),
    (wgpu::Backend::Vulkan, Api::Vulkan),
    (wgpu::Backend::Dx12, Api::Dx12),
    (wgpu::Backend::BrowserWebGpu, Api::Webgpu),
];

/// An adapter of `kind` on `backend`, with a driver named so the header reads no `sw_vers`.
fn adapter(kind: wgpu::DeviceType, backend: wgpu::Backend) -> wgpu::AdapterInfo {
    let mut info = wgpu::AdapterInfo::new(kind, backend);
    info.name = "qa adapter".to_owned();
    info.driver = "qa driver".to_owned();
    info
}

/// For each recorded backend, `api` of an adapter reads the header's API, for an integrated adapter (which the header
/// takes, and must agree) and a discrete one (which it refuses); for a backend the header doesn't record, none.
fn check_api_of(api: impl Fn(&wgpu::AdapterInfo) -> Option<Api>) {
    for (backend, want) in RECORDED {
        let integrated = adapter(wgpu::DeviceType::IntegratedGpu, backend);
        let header = session_adapter(&integrated, wgpu::Features::empty())
            .expect("the header takes an integrated adapter")
            .api;
        assert_eq!(header, want, "the header's own API for {backend:?}");
        assert_eq!(
            api(&integrated),
            Some(want),
            "an integrated {backend:?} adapter's API is misread"
        );
        let discrete = adapter(wgpu::DeviceType::DiscreteGpu, backend);
        assert!(
            session_adapter(&discrete, wgpu::Features::empty()).is_err(),
            "the header no longer refuses a discrete {backend:?} adapter"
        );
        assert_eq!(
            api(&discrete),
            Some(want),
            "a discrete {backend:?} adapter's API is misread"
        );
    }
    for backend in [wgpu::Backend::Gl, wgpu::Backend::Noop] {
        assert_eq!(
            api(&adapter(wgpu::DeviceType::IntegratedGpu, backend)),
            None,
            "{backend:?} read as an API the header records"
        );
    }
}

#[test]
fn qa_the_api_comes_from_the_opened_backend() {
    check_api_of(|info| header_api(info.backend));
}

negative_control!(
    qa_the_api_comes_from_the_opened_backend,
    "the API read through the session header, as the fast-math checks read it before",
    expected = "a discrete Metal adapter's API is misread",
    check_api_of(|info| session_adapter(info, wgpu::Features::empty())
        .ok()
        .map(|a| a.api))
);
