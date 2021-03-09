pub trait Chip: Send + Sync {
    type InputPins: Send + Sync;
    type OutputPins: Send + Sync;

    fn clock(&mut self, input: Self::InputPins) -> Self::OutputPins;
}
