use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, SampleFormat, SampleRate, SupportedBufferSize, SupportedStreamConfig,
};

use rustfft::{num_complex::Complex, num_traits::Pow, FftPlanner};

pub type Raw = Arc<Mutex<VecDeque<f32>>>;
pub type Frequencies = Arc<Mutex<VecDeque<Option<f32>>>>;

const MAX_FREQUENCIES: usize = 500;

#[derive(Default, Clone)]
pub struct Vocalize {
    pub raw: Raw,
    pub frequencies: Frequencies,
    pub frequencies_postprocessed: Frequencies,
}

impl Vocalize {
    pub fn get_values(self) -> VecDeque<Option<f32>> {
        let fr = self.frequencies_postprocessed.lock().unwrap();
        fr.clone()
    }

    pub fn new() -> Self {
        let raw: Raw = Arc::new(Mutex::new(VecDeque::new()));
        let frequencies: Frequencies =
            Arc::new(Mutex::new(VecDeque::from(vec![None; MAX_FREQUENCIES])));
        let frequencies_postprocessed: Frequencies =
            Arc::new(Mutex::new(VecDeque::from(vec![None; MAX_FREQUENCIES])));
        Vocalize {
            raw,
            frequencies,
            frequencies_postprocessed,
        }
    }

    fn postprocess(before: Frequencies, after: Frequencies) {
        let window_size = 5;

        let before = before.lock().unwrap();
        let count = before
            .iter()
            .rev()
            .take(window_size)
            .filter(|a| a.is_some())
            .count();

        let mut after = after.lock().unwrap();
        after.rotate_left(1);
        after[before.len() - 1] = before[before.len() - 1];
        if count > 3 {
            let sum: f32 = before
                .iter()
                .rev()
                .take(window_size)
                .filter(|a| a.is_some())
                .map(|a| a.unwrap())
                .sum();
            let mean = sum / count as f32;
            let sum_variance: f32 = before
                .iter()
                .rev()
                .take(window_size)
                .filter(|a| a.is_some())
                .map(|a| {
                    let a = a.unwrap();
                    (a - mean).pow(2)
                })
                .sum();
            let mean_variance = sum_variance / ((count - 1) as f32);
            let standard_deviation = mean_variance.sqrt();

            if let Some(_a) = before[before.len() - 1] {
                let centered_reduced = (_a - mean) / standard_deviation;
                after[before.len() - 1] = Some(_a);
                if centered_reduced > 2.0 {
                    after[before.len() - 1] = None;
                }
            }
        }
    }

    pub fn run(&self) {
        let host = cpal::default_host();
        let device = host.default_input_device().unwrap();
        println!("Input device: {}", device.name().unwrap());

        let config = SupportedStreamConfig::new(
            1,
            SampleRate(384000),
            SupportedBufferSize::Range {
                min: 3,
                max: 4194304,
            },
            SampleFormat::F32,
        );
        println!("Input config: {:?}", config);

        println!("Begin recording...");

        println!("Vocalize started");
        {
            let raw = self.raw.clone();
            let frequencies = self.frequencies.clone();
            let frequencies_postprocessed = self.frequencies_postprocessed.clone();
            let sample_rate = config.sample_rate().0;
            thread::spawn(move || {
                let err_fn = move |err| {
                    eprintln!("an error occurred on stream: {}", err);
                };
                let rw = raw.clone();
                let stream = match config.sample_format() {
                    cpal::SampleFormat::F32 => device
                        .build_input_stream(
                            &config.clone().into(),
                            move |data, info| Vocalize::write_input_data(data, info, rw.clone()),
                            err_fn,
                            None,
                        )
                        .unwrap(),
                    sample_format => {
                        eprintln!("Unsupported sample format '{sample_format}'");
                        return;
                    }
                };
                stream.play().unwrap();
                let sample_size = (384000.0 / 50.0) as usize * 20;
                let freq_step = sample_rate as f32 / sample_size as f32;
                println!("Sample rate: {}", sample_rate);
                println!("Sample size: {}", sample_size);
                println!("Frequency steps: {}Hz", freq_step);
                loop {
                    thread::sleep(Duration::from_millis(1000 / 140));
                    let rw = raw.lock().unwrap();
                    if rw.len() >= sample_size {
                        let mut buffer: Vec<Complex<f32>> = rw
                            .iter()
                            .rev()
                            .take(sample_size)
                            .rev()
                            .map(|el| Complex {
                                re: *el,
                                im: 0.0f32,
                            })
                            .collect();
                        drop(rw);

                        let mut planner = FftPlanner::<f32>::new();
                        let fft = planner.plan_fft_forward(buffer.len());
                        fft.process(&mut buffer);
                        let norms: Vec<(usize, f32)> = buffer
                            .iter()
                            .take(buffer.len() / 2)
                            .map(|freq| ((freq.re * freq.re + freq.im * freq.im) as f32).sqrt())
                            .enumerate()
                            .collect();

                        let max_peak = norms.iter().max_by_key(|(_i, norm)| *norm as u32).unwrap();

                        let freq = max_peak.0 as f32 * freq_step;
                        if max_peak.1 > 5.0 {
                            println!(
                                "{:>9}Hz \t\t {:>4}/{:>4} \t\t\t max: {:0<11}",
                                freq,
                                max_peak.0 + 1,
                                buffer.len() / 2,
                                max_peak.1,
                            );
                            let mut fr = frequencies.lock().unwrap();
                            fr.push_back(Some(freq));
                        } else {
                            let mut fr = frequencies.lock().unwrap();
                            fr.push_back(None);
                        }
                    } else {
                        println!("Sample size too small: {}", rw.len());
                        let mut fr = frequencies.lock().unwrap();
                        fr.push_back(None);
                    }

                    let mut fr = frequencies.lock().unwrap();
                    while fr.len() > MAX_FREQUENCIES {
                        fr.pop_front();
                    }
                    drop(fr);

                    Vocalize::postprocess(frequencies.clone(), frequencies_postprocessed.clone());
                }
            });
        }
    }

    fn write_input_data(input: &[f32], _info: &InputCallbackInfo, raw: Raw) {
        let mut temp: VecDeque<f32> = input.to_vec().into();
        let mut rw = raw.lock().unwrap();
        rw.append(&mut temp);
        while rw.len() > 200000 {
            rw.pop_front();
        }
    }
}
