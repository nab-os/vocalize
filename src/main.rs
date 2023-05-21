#![allow(non_snake_case)]
use std::time::Duration;

use dioxus::prelude::*;

use tokio::time::sleep;
use vocalize::Vocalize;

const MAX: f32 = 450.0;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    let svg_style = r"
        html {
            width: 100vw;
            height: 100vh;
            margin: 0px;
        }

        body, #main, svg {
            width: 100%;
            height: 100%;
            margin: 0px;
        }
    ";

    let lines = use_state(cx, || Vec::new());
    use_coroutine(cx, |_: UnboundedReceiver<()>| {
        let lines = lines.to_owned();
        async move {
            let vocalize = Box::new(Vocalize::new());
            vocalize.run();
            loop {
                sleep(Duration::from_millis(1000 / 60)).await;
                lines.set(
                    vocalize
                        .clone()
                        .get_values()
                        .iter()
                        .enumerate()
                        .fold(Vec::<Vec<(usize, String)>>::new(), |mut acc, (i, v)| {
                            if acc.is_empty() {
                                acc.push(Vec::<(usize, String)>::new())
                            }
                            match v {
                                Some(v) => acc.last_mut().unwrap().push((i, (MAX - v).to_string())),
                                None => acc.push(Vec::<(usize, String)>::new()),
                            }
                            acc
                        })
                        .iter()
                        .map(|l| {
                            let temp: Vec<String> =
                                l.iter().map(|(a, b)| format!("{} {}", a, b)).collect();
                            temp.join(" C")
                        })
                        .filter(|l| !l.is_empty())
                        .collect(),
                );
            }
        }
    });

    cx.render(rsx! {
        style { svg_style }
        svg {
            preserve_aspect_ratio: "xMidYMid meet",
            fill: "none",
            stroke: "red",
            xmlns: "http://www.w3.org/2000/svg",
            "viewBox": "0 0 600 {MAX}",

            //Back
            g {
                rect {
                    x: "0",
                    y: "{MAX - 180.0 - 125.0}",
                    width: "600",
                    height: "125",
                    style: "fill: rgb(245 169 184); stroke: none",
                },
                rect {
                    x: "0",
                    y: "{MAX - 80.0 - 80.0}",
                    width: "600",
                    height: "85",
                    style: "fill: rgb(91 206 250); stroke: none",
                },

                for y in (50..(MAX as usize -50)).step_by(50) {
                    rsx! {
                        line {
                            x1: "0",
                            y1: "{MAX - y as f32}",
                            x2: "600",
                            y2: "{MAX - y as f32}",
                            style: "stroke: gray; stroke-with: 1;",
                        },
                        text {
                            x: "570",
                            y: "{MAX - y as f32 - 2.0}",
                            style: "fill: gray; stroke: none",
                            y.to_string(),
                        }
                    }
                }
            }

            //Middle
            g {
                lines.iter().map(|l|
                    rsx!{
                        path {
                            "stroke-linecap": "round",
                            "stroke-width": "2",
                            "stroke-linejoin": "round",
                            d: "M{l.clone()}",
                        }
                    }
                )
            }

            //Front
            g {
                text {
                    x: "5",
                    y: "{MAX - 180.0 - 125.0 + 33.0}",
                    style: "fill: white; stroke: none",
                    "Hauteur féminine"
                }
                text {
                    x: "5",
                    y: "{MAX - 80.0 - 85.0 + 47.0}",
                    style: "fill: white; stroke: none",
                    "Hauteur masculine"
                }
            }
        }
    })
}
