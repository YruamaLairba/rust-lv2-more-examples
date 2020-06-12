use lv2_core::prelude::*;
use urid::*;

#[derive(PortCollection)]
struct Ports {
    _param1: InputPort<Control>,
    _param2: InputPort<Control>,
    _param3: InputPort<Control>,
}

/// A plugin to demonstrate how to make preset. This is fully handled by rdf spec, so the plugin
/// does nothing.
#[uri("urn:rust-lv2-more-examples:eg-preset-rs")]
struct EgPreset {}

impl Plugin for EgPreset {
    type Ports = Ports;
    type InitFeatures = ();
    type AudioFeatures = ();

    fn new(_plugin_info: &PluginInfo, _features: &mut Self::InitFeatures) -> Option<Self> {
        Some(Self {})
    }

    fn run(&mut self, _ports: &mut Ports, _features: &mut Self::AudioFeatures) {}
}

lv2_descriptors!(EgPreset);
