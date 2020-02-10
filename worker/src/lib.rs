use lv2_core::prelude::*;
use lv2_sys::{
    LV2_Handle, LV2_WORKER__interface, LV2_Worker_Interface, LV2_Worker_Respond_Function,
    LV2_Worker_Respond_Handle, LV2_Worker_Status, LV2_Worker_Status_LV2_WORKER_ERR_NO_SPACE,
    LV2_Worker_Status_LV2_WORKER_ERR_UNKNOWN, LV2_Worker_Status_LV2_WORKER_SUCCESS,
};
use std::os::raw::*; //get all commoent c_type

// see lv2_core::port::PortContainer for port type
#[derive(PortContainer)]
struct Ports {
    gain: InputPort<Control>,
    input: InputPort<Audio>,
    output: OutputPort<Audio>,
}

struct EgWorker;

// URI identifier
unsafe impl UriBound for EgWorker {
    const URI: &'static [u8] = b"urn:rust-lv2-more-examples:eg-worker-rs\0";
}

impl Plugin for EgWorker {
    type Ports = Ports;
    type Features = ();

    fn new(_plugin_info: &PluginInfo, _features: ()) -> Option<Self> {
        Some(Self)
    }

    fn activate(&mut self) {}

    fn deactivate(&mut self) {}

    fn run(&mut self, ports: &mut Ports) {
        let coef = if *(ports.gain) > -90.0 {
            10.0_f32.powf(*(ports.gain) * 0.05)
        } else {
            0.0
        };

        for (in_frame, out_frame) in Iterator::zip(ports.input.iter(), ports.output.iter_mut()) {
            *out_frame = in_frame * coef;
        }
    }

    fn extension_data(_uri: &Uri) -> Option<&'static dyn Any> {
        static WORKER: LV2_Worker_Interface = LV2_Worker_Interface {
            work: Some(work),
            work_response: None,
            end_run: None,
        };
        if _uri.to_bytes_with_nul()[..] == LV2_WORKER__interface[..] {
            return Some(&WORKER);
        } else {
            return None;
        }
    }
}

// Worker spec implementation

extern "C" fn work(
    _instance: LV2_Handle,
    _respond: LV2_Worker_Respond_Function,
    _handle: LV2_Worker_Respond_Handle,
    _size: u32,
    _data: *const c_void,
) -> LV2_Worker_Status {
    return LV2_Worker_Status_LV2_WORKER_SUCCESS;
}

lv2_descriptors!(EgWorker);
