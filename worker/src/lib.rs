use core::any::Any;
use lv2_core::extension::ExtensionDescriptor;
use lv2_core::prelude::*;
use lv2_sys::{
    LV2_Handle, LV2_WORKER__interface, LV2_Worker_Interface, LV2_Worker_Respond_Function,
    LV2_Worker_Respond_Handle, LV2_Worker_Status, LV2_Worker_Status_LV2_WORKER_ERR_NO_SPACE,
    LV2_Worker_Status_LV2_WORKER_ERR_UNKNOWN, LV2_Worker_Status_LV2_WORKER_SUCCESS,
};
use std::marker::PhantomData;
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

    fn extension_data(uri: &Uri) -> Option<&'static dyn Any> {
        match_extensions![uri, WorkerDescriptor<Self>]
    }
}

//type
pub enum WorkerStatus {
    Success,
    Unknown,
    NoSpace,
}

// Worker Traits
pub trait Worker: Plugin {
    fn work(
        &mut self,
        response_function: LV2_Worker_Respond_Function,
        respond_handle: LV2_Worker_Respond_Handle,
        size: u32,
        data: *const c_void,
    ) -> WorkerStatus;
}

// A descriptor for the plugin. This is just a marker type to associate constants and methods with.
pub struct WorkerDescriptor<P: Worker> {
    plugin: PhantomData<P>,
}

#[repr(C)]
/// This struct would be part of a sys crate.
pub struct WorkerInterface {
    work: unsafe extern "C" fn(
        LV2_Handle,
        LV2_Worker_Respond_Function,
        LV2_Worker_Respond_Handle,
        u32,
        *const c_void,
    ) -> LV2_Worker_Status,
}

unsafe impl<P: Worker> UriBound for WorkerDescriptor<P> {
    const URI: &'static [u8] = LV2_WORKER__interface;
}

impl<P: Worker> WorkerDescriptor<P> {
    /// The extern, unsafe version of the extending method.
    ///
    /// This is actually called by the host.
    unsafe extern "C" fn extern_work(
        handle: LV2_Handle,
        response_function: LV2_Worker_Respond_Function,
        respond_handle: LV2_Worker_Respond_Handle,
        size: u32,
        data: *const c_void,
    ) -> LV2_Worker_Status {
        let plugin = (handle as *mut P).as_mut().unwrap();
        match plugin.work(response_function, respond_handle, size, data) {
            WorkerStatus::Success => LV2_Worker_Status_LV2_WORKER_SUCCESS,
            WorkerStatus::Unknown => LV2_Worker_Status_LV2_WORKER_ERR_UNKNOWN,
            WorkerStatus::NoSpace => LV2_Worker_Status_LV2_WORKER_ERR_NO_SPACE,
        }
    }
}

// Implementing the trait that contains the interface.
impl<P: Worker> ExtensionDescriptor for WorkerDescriptor<P> {
    type ExtensionInterface = WorkerInterface;

    const INTERFACE: &'static WorkerInterface = &WorkerInterface {
        work: Self::extern_work,
    };
}

// Actually implementing the extension.
impl Worker for EgWorker {
    fn work(
        &mut self,
        _response_function: LV2_Worker_Respond_Function,
        _respond_handle: LV2_Worker_Respond_Handle,
        _size: u32,
        _data: *const c_void,
    ) -> WorkerStatus {
        return WorkerStatus::Success;
    }
}

lv2_descriptors!(EgWorker);
