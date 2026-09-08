use serde_json::{Value, json};

pub const PROTOCOL: &str = "rackforge.plugin.web@1";
/// Gain, pickup law, distance, alignment, hardness, sustain, bell, dynamics.
pub const PARAMETERS: usize = 8;
pub const DEFAULTS: [f64; PARAMETERS] = [0.1, 0.0, 1.5, 0.5, 0.5, 0.0, 1.0, 0.5];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    Fetch,
    Set(usize, f64),
}

pub struct Client {
    pub values: [f64; PARAMETERS],
    pub loaded: bool,
    pub status: String,
    queued: [Option<f64>; PARAMETERS],
    pending: Option<(String, Operation, f64)>,
    serial: u64,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            values: DEFAULTS,
            loaded: false,
            status: "Connecting to RackForge...".into(),
            queued: [None; PARAMETERS],
            pending: None,
            serial: 0,
        }
    }
}

pub fn valid(index: usize, value: f64) -> bool {
    value.is_finite()
        && match index {
            0 => (0.0..=2.0).contains(&value),
            1 => [0.0, 1.0].contains(&value),
            2 => (0.5..=3.0).contains(&value),
            3 => (-1.0..=1.5).contains(&value),
            4..=7 => (0.0..=1.0).contains(&value),
            _ => false,
        }
}

impl Client {
    pub fn queue(&mut self, index: usize, value: f64) {
        if self.loaded && valid(index, value) {
            self.queued[index] = Some(value);
        }
    }

    pub fn next(&mut self, now: f64, poll: bool) -> Option<Value> {
        if self
            .pending
            .as_ref()
            .is_some_and(|(_, _, sent)| now - sent > 5000.0)
        {
            self.pending = None;
            self.queued.fill(None);
            self.loaded = false;
            self.status = "Host timed out. Reconnecting...".into();
        }
        if self.pending.is_some() {
            return None;
        }
        let operation = if let Some(index) = self.queued.iter().position(Option::is_some) {
            Operation::Set(index, self.queued[index].take().expect("queued value"))
        } else if poll {
            Operation::Fetch
        } else {
            return None;
        };
        self.serial = self.serial.wrapping_add(1);
        let id = format!("rf-73-ui-{}", self.serial);
        self.pending = Some((id.clone(), operation, now));
        let (method, params) = match operation {
            Operation::Fetch => ("plugin.parameters", json!({})),
            Operation::Set(index, value) => (
                "plugin.set_parameter",
                json!({"parameter_index": index, "value": value}),
            ),
        };
        Some(
            json!({"protocol": PROTOCOL, "kind": "request", "request_id": id, "method": method, "params": params}),
        )
    }

    pub fn response(&mut self, message: &Value) {
        let Some((id, operation, _)) = &self.pending else {
            return;
        };
        if message["request_id"].as_str() != Some(id) {
            return;
        }
        let operation = *operation;
        self.pending = None;
        if message["ok"].as_bool() != Some(true) {
            self.queued.fill(None);
            self.loaded = false;
            self.status = "Host rejected the request. Reconnecting...".into();
            return;
        }
        let updated = match operation {
            Operation::Set(index, _) => message["result"]["value"]
                .as_f64()
                .filter(|value| valid(index, *value))
                .map(|value| {
                    let mut values = self.values;
                    values[index] = value;
                    values
                }),
            Operation::Fetch => snapshot(&message["result"]),
        };
        if let Some(values) = updated {
            self.values = values;
            self.loaded = true;
            self.status = "Connected to RackForge".into();
        } else {
            self.loaded = false;
            self.queued.fill(None);
            self.status = "Invalid host parameter response. Reconnecting...".into();
        }
    }

    pub fn display(&self, index: usize) -> f64 {
        self.queued[index]
            .or_else(|| match self.pending.as_ref().map(|(_, op, _)| *op) {
                Some(Operation::Set(i, value)) if i == index => Some(value),
                _ => None,
            })
            .unwrap_or(self.values[index])
    }
}

fn snapshot(result: &Value) -> Option<[f64; PARAMETERS]> {
    let mut values = [None; PARAMETERS];
    for entry in result["values"].as_array()? {
        let index = usize::try_from(entry["index"].as_u64()?).ok()?;
        if index >= PARAMETERS {
            continue;
        }
        let value = entry["value"].as_f64()?;
        if !valid(index, value) || values[index].replace(value).is_some() {
            return None;
        }
    }
    let mut complete = [0.0; PARAMETERS];
    for (slot, value) in complete.iter_mut().zip(values) {
        *slot = value?;
    }
    Some(complete)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reply(id: &Value, result: Value) -> Value {
        json!({"request_id": id["request_id"], "ok": true, "result": result})
    }
    fn connect(client: &mut Client) {
        let request = client.next(0.0, true).unwrap();
        let values: Vec<Value> = DEFAULTS
            .iter()
            .enumerate()
            .map(|(i, v)| json!({"index": i, "value": v}))
            .collect();
        client.response(&reply(&request, json!({"values": values})));
        assert!(client.loaded);
    }
    #[test]
    fn edits_coalesce_and_serialize_without_stale_acknowledgements_overwriting_them() {
        let mut client = Client::default();
        connect(&mut client);
        client.queue(0, 0.2);
        let first = client.next(1.0, false).unwrap();
        client.queue(0, 0.3);
        client.queue(0, 0.4);
        client.queue(5, 1.0);
        assert!(client.next(2.0, true).is_none());
        client.response(&reply(&first, json!({"value": 0.2})));
        assert_eq!(client.display(0), 0.4);
        let second = client.next(3.0, false).unwrap();
        assert_eq!(second["params"]["value"], 0.4);
        client.response(&reply(&first, json!({"value": 0.2})));
        assert!(client.next(4.0, false).is_none());
        client.response(&reply(&second, json!({"value": 0.4})));
        assert_eq!(
            client.next(5.0, false).unwrap()["params"]["parameter_index"],
            5
        );
    }
    #[test]
    fn timeout_discards_ambiguous_writes_and_recovers_through_a_fresh_snapshot() {
        let mut client = Client::default();
        connect(&mut client);
        client.queue(3, 1.0);
        let old = client.next(1.0, false).unwrap();
        client.queue(1, 1.0);
        let fresh = client.next(6000.0, true).unwrap();
        assert_eq!(fresh["method"], "plugin.parameters");
        assert!(!client.loaded);
        client.response(&reply(&old, json!({"value": 1.0})));
        assert!(!client.loaded);
        client.response(&reply(&fresh, json!({"values":[]})));
        assert!(!client.loaded);
    }
    #[test]
    fn snapshots_require_all_parameters_with_valid_domains_and_no_duplicates() {
        assert!(snapshot(&json!({"values":[{"index":1,"value":0.5}]})).is_none());
        assert!(!valid(1, 0.5));
        assert!(!valid(0, f64::NAN));
        assert!(!valid(8, 0.0));
        assert!(valid(2, 0.5) && !valid(2, 0.4) && valid(3, -1.0) && !valid(3, 1.6));
        assert!(valid(7, 1.0) && !valid(4, 1.5));
        let mut client = Client::default();
        client.queue(0, 0.9);
        assert!(client.next(0.0, false).is_none());
    }
}
