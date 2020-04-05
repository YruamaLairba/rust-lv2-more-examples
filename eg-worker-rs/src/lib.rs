use core::any::Any;
use lv2_core::feature::*;
use lv2_core::prelude::*;
use lv2_worker::*;
use urid::*;

#[derive(PortCollection)]
struct Ports {
    trigger_task: InputPort<Control>,
}

/// Requested features
#[derive(FeatureCollection)]
struct AudioFeatures<'a> {
    ///host feature allowing to schedule some work
    schedule: Schedule<'a, EgWorker>,
}

//Data type for scheduling work
enum Task {
    Say(&'static str),
}

/// A plugin that do some work in another thread
#[uri("urn:rust-lv2-more-examples:eg-worker-rs")]
struct EgWorker {
    //false for off, true for on
    last_trigger_task: bool,
}

impl Plugin for EgWorker {
    type Ports = Ports;
    type InitFeatures = ();
    type AudioFeatures = AudioFeatures<'static>;

    fn new(_plugin_info: &PluginInfo, _features: &mut Self::InitFeatures) -> Option<Self> {
        Some(Self {
            last_trigger_task: false,
                    })
    }

    fn run(&mut self, ports: &mut Ports, features: &mut Self::AudioFeatures) {
        if *ports.trigger_task > 0f32 && !self.last_trigger_task {
            self.last_trigger_task = true;
            let message = Task::Say("New task triggered");
            let _ = features.schedule.schedule_work(message);
        } else if *ports.trigger_task <= 0f32 && self.last_trigger_task {
            self.last_trigger_task = false;
        }
    }

    fn extension_data(uri: &Uri) -> Option<&'static dyn Any> {
        match_extensions![uri, WorkerDescriptor<Self>]
    }
}

// Actually implementing the extension.
impl Worker for EgWorker {
    /// data type sended by the schedule handler and received by the `work` method.
    type WorkData = Task;
    /// data type sended by the response handler and received by the `work_response` method.
    type ResponseData = Result<(),()>;
    fn work(
        //response handler is associated to the plugin type.
        response_handler: &ResponseHandler<Self>,
        received_data: Self::WorkData,
    ) -> Result<(), WorkerError> {
        match received_data {
            Task::Say(message) => {
                println!("{}", message);
                let _ = response_handler.respond(Ok(()));
                Ok(())
            },
        }
    }

    fn work_response(
        &mut self,
        data: Self::ResponseData,
        _features: &mut Self::AudioFeatures,
    ) -> Result<(), WorkerError> {
        if let Err(()) = data {
            //printing should normally be avoided in the audio thread
            println!("oops work returned an error")
        }
        Ok(())
    }
}

lv2_descriptors!(EgWorker);
