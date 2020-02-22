use core::any::Any;
use lv2_core::extension::ExtensionDescriptor;
use lv2_core::feature::*;
use lv2_core::prelude::*;
use lv2_urid::prelude::*;
use lv2_sys;
use lv2_sys::{
    LV2_Handle,
    LV2_WORKER__interface,
    LV2_Worker_Interface,
    LV2_Worker_Respond_Function,
    LV2_Worker_Respond_Handle,
    LV2_Worker_Schedule,
    LV2_Worker_Status,
    LV2_Worker_Status_LV2_WORKER_ERR_NO_SPACE,
    LV2_Worker_Status_LV2_WORKER_ERR_UNKNOWN,
    LV2_Worker_Status_LV2_WORKER_SUCCESS,
};
use lv2_worker::*;
use std::marker::PhantomData;
use std::os::raw::*; //get all common c_type



// see lv2_core::port::PortContainer for port type
#[derive(PortContainer)]
struct Ports {
    gain: InputPort<Control>,
    input: InputPort<Audio>,
    output: OutputPort<Audio>,
}

#[derive(FeatureCollection)]
pub struct Features<'a> {
    map: Map<'a>,
    schedule: Schedule<'a>,
}

#[repr(transparent)]
struct WorkerSchedule {
    internal: LV2_Worker_Schedule,
}
//lying on sync and send
unsafe impl Sync for WorkerSchedule {}
unsafe impl Send for WorkerSchedule {}

#[repr(C)]
struct EgWorker<'a>
{
    schedule: Schedule<'a>,
}

// URI identifier
unsafe impl<'a> UriBound for EgWorker<'a> {
    const URI: &'static [u8] = b"urn:rust-lv2-more-examples:eg-worker-rs\0";
}

impl Plugin for EgWorker<'static> {
    type Ports = Ports;
    type Features = Features<'static>;

    fn new(_plugin_info: &PluginInfo, features: Features<'static>) -> Option<Self> {
        //match features.map.map_type::<Schedule>() {
        //    Some(x) => println!("Schedule feature {:?}",x),
        //    None => println!("No Schedule feature"),
        //}
        let schedule = features.schedule;
        Some(Self {
            schedule,
        })
    }

    fn activate(&mut self) {}

    fn deactivate(&mut self) {}

    fn run(&mut self, ports: &mut Ports) {
        let _ = self.schedule.schedule_work(0, std::ptr::null::<c_void>() as *const std::ffi::c_void);
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

// Actually implementing the extension.
impl Worker for EgWorker<'static> {
    fn work(
        &mut self,
        response_handler: &ResponseHandler,
        size: u32,
        data: *const c_void,
    ) -> Result<(), WorkerError> {
        println!("worker thread");
        let _ = response_handler.respond(size, data);
        return Ok(());
    }

    fn work_response(&mut self, _size: u32, _body: *const c_void) -> Result<(),WorkerError> {
        println!("work response");
        return Ok(());
    }

}

lv2_descriptors!(EgWorker<'static>);
