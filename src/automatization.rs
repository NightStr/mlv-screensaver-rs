pub struct Notifier{
    low_hp_alert: &'static str,
    high_hp_alert: &'static str,
    volume: f32
}

impl Notifier {
    pub fn new(volume: f32, low_hp_alert: &'static str, high_hp_alert: &'static str) -> Self {
        Notifier{
            low_hp_alert,
            high_hp_alert,
            volume
        }
    }

    fn notify(&self, file: std::fs::File) -> Result<(), &str> {
        let (_stream, handle) = rodio::OutputStream::try_default().expect(
            "Failed to get default output stream"
        );
        let sink = rodio::Sink::try_new(&handle).unwrap();
        sink.append(
            rodio::Decoder::new(
                std::io::BufReader::new(file)
            ).expect("Failed to create decoder")
        );

        sink.set_volume(self.volume);
        sink.sleep_until_end();
        Ok(())
    }

    pub fn low_hp_notify(&mut self) -> Result<(), &str> {
        self.notify(
            std::fs::File::open(self.low_hp_alert).expect("Failed to open low hp file")
        )
    }

    pub fn high_hp_notify(&mut self) -> Result<(), &str> {
        self.notify(
            std::fs::File::open(self.high_hp_alert).expect("Failed to open high hp file")
        )
    }
}

pub struct AutoClicker{}