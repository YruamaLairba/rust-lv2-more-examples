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
struct EgWorker {
    schedule_handler: ScheduleHandler<Self>,
}

// URI identifier
unsafe impl UriBound for EgWorker {
    const URI: &'static [u8] = b"urn:rust-lv2-more-examples:eg-worker-rs\0";
}

impl Plugin for EgWorker {
    type Ports = Ports;
    type Features = Features<'static>;

    fn new(_plugin_info: &PluginInfo, features: Features<'static>) -> Option<Self> {
        let schedule_handler = ScheduleHandler::from(features.schedule);
        Some(Self { schedule_handler })
    }

    fn activate(&mut self) {}

    fn deactivate(&mut self) {}

    fn run(&mut self, ports: &mut Ports) {
        let work = String::from("This is data to work on");
        let _ = self
            .schedule_handler
            .schedule_work(work);
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
impl Worker for EgWorker {
    // be carefull, using associated type that allocate breaks HardRtCapability most of the time
    type WorkData = String;
    type ResponseData = Vec<&'static str>;
    fn work(
        &mut self,
        response_handler: &ResponseHandler<Self>,
        data: Self::WorkData,
    ) -> Result<(), WorkerError> {
        println!("worker thread: {:?}", data);
        let _ = response_handler.respond(vec![&"This",&"is",&"the",&"worker",&"result"]);
        return Ok(());
    }

    fn work_response(&mut self, data: Self::ResponseData) -> Result<(), WorkerError> {
        println!("work response: {:?}",data);
        return Ok(());
    }

    fn end_run(&mut self)-> Result<(), WorkerError> {
        println!("end run");
        return Ok(());
    }
}

lv2_descriptors!(EgWorker);
