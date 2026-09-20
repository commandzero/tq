//! Structural-event correctness oracle and constant-space timed consumer.
use serde_json::Value as Json;
use std::{convert::Infallible, hint::black_box};
use tq_toon::{Event, EventConsumer, Scalar};

#[derive(Default)]
pub struct Consume {
    pub count: u64,
}
impl EventConsumer for Consume {
    type Error = Infallible;
    fn prefers_decoded_events(&self) -> bool {
        true
    }

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        black_box(event);
        self.count += 1;
        Ok(())
    }
}

#[derive(Default)]
pub struct Check {
    pub events: Vec<String>,
}
impl EventConsumer for Check {
    type Error = Infallible;
    fn prefers_decoded_events(&self) -> bool {
        true
    }

    fn consume(&mut self, event: Event) -> Result<(), Self::Error> {
        let token = match event {
            Event::DocumentStart { .. } => "document".to_owned(),
            Event::DocumentEnd { .. } => "/document".to_owned(),
            Event::ObjectStart { .. } => "object".to_owned(),
            Event::ObjectEnd { .. } => "/object".to_owned(),
            Event::ArrayStart { .. } => "array".to_owned(),
            Event::ArrayEnd { observed_count, .. } => format!("/array:{observed_count}"),
            Event::Key { value, .. } => format!("key:{value}"),
            Event::Scalar { value, .. } => match value {
                Scalar::Null => "value:null".to_owned(),
                Scalar::Bool(value) => format!("value:{value}"),
                Scalar::Number(value) => format!("value:{value}"),
                Scalar::String(value) => format!("string:{value}"),
            },
        };
        self.events.push(token);
        Ok(())
    }
}

pub fn expected(model: &Json) -> Vec<String> {
    fn visit(model: &Json, output: &mut Vec<String>) {
        match model {
            Json::Object(object) => {
                output.push("object".into());
                for (key, value) in object {
                    output.push(format!("key:{key}"));
                    visit(value, output);
                }
                output.push("/object".into());
            }
            Json::Array(array) => {
                output.push("array".into());
                for value in array {
                    visit(value, output);
                }
                output.push(format!("/array:{}", array.len()));
            }
            Json::String(value) => output.push(format!("string:{value}")),
            value => output.push(format!("value:{value}")),
        }
    }
    let mut output = vec!["document".into()];
    visit(model, &mut output);
    output.push("/document".into());
    output
}
