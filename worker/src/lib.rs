use core::any::Any;
use lv2_core::feature::*;
use lv2_core::prelude::*;
use lv2_worker::*;

// see lv2_core::port::PortContainer for port type
#[derive(PortContainer)]
struct Ports {
    gain: InputPort<Control>,
    input: InputPort<Audio>,
    output: OutputPort<Audio>,
}

#[derive(FeatureCollection)]
pub struct Features<'a> {
    schedule: Schedule<'a>,
}

#[repr(C)]
struct EgWorker<'a> {
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
        Some(Self { schedule })
    }

    fn activate(&mut self) {}

    fn deactivate(&mut self) {}

    fn run(&mut self, ports: &mut Ports) {
        let _ = self
            .schedule
            .schedule_work::<Self>(&32);
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
    type WorkData = u32;
    type ResponseData = &'static str;
    fn work(
        &mut self,
        response_handler: &ResponseHandler,
        data: &u32,
    ) -> Result<(), WorkerError> {
        println!("worker thread: got {}", data);
        let _ = response_handler.respond::<Self>(&"hello");
        return Ok(());
    }

    fn work_response(&mut self, data: &&str) -> Result<(), WorkerError> {
        println!("work response: got {}", data);
        return Ok(());
    }
}

lv2_descriptors!(EgWorker<'static>);
