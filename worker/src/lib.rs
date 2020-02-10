use lv2_core::prelude::*;

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
}

lv2_descriptors!(EgWorker);
