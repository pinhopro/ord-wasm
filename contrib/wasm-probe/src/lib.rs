use bitcoin::{Amount, ScriptBuf, Transaction, TxOut, absolute, consensus, transaction};
use ordinals::{Artifact, Edict, Etching, Rune, RuneId, Runestone, Sat, Terms, varint};
use serde_json::{Value, json};

pub const MAX_INPUT_BYTES: usize = 65_536;

pub fn encode_hex(bytes: &[u8]) -> String {
  bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
  if !value.len().is_multiple_of(2) || !value.is_ascii() {
    return Err("hex must contain complete ASCII byte pairs".into());
  }
  value
    .as_bytes()
    .chunks_exact(2)
    .map(|pair| {
      let text = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
      u8::from_str_radix(text, 16).map_err(|error| error.to_string())
    })
    .collect()
}

fn field<'a>(request: &'a Value, name: &str) -> Result<&'a str, String> {
  request[name]
    .as_str()
    .ok_or_else(|| format!("{name} must be a string"))
}

fn unsigned(request: &Value, name: &str) -> Result<u128, String> {
  field(request, name)?
    .parse::<u128>()
    .map_err(|error| error.to_string())
}

pub fn transaction_with_script(script: ScriptBuf, output_count: usize) -> Transaction {
  let mut output = vec![
    TxOut {
      value: Amount::ZERO,
      script_pubkey: ScriptBuf::new()
    };
    output_count
  ];
  output[0].script_pubkey = script;
  Transaction {
    version: transaction::Version::TWO,
    lock_time: absolute::LockTime::ZERO,
    input: vec![],
    output,
  }
}

fn describe_artifact(artifact: Option<Artifact>) -> Value {
  match artifact {
    None => json!({ "kind": "none" }),
    Some(Artifact::Cenotaph(cenotaph)) => json!({
        "kind": "cenotaph",
        "flaw": cenotaph.flaw.map(|flaw| format!("{flaw:?}")),
    }),
    Some(Artifact::Runestone(runestone)) => {
      let edicts: Vec<Value> = runestone
        .edicts
        .iter()
        .map(|edict| {
          json!({
              "block": edict.id.block.to_string(), "tx": edict.id.tx.to_string(),
              "amount": edict.amount.to_string(), "output": edict.output,
          })
        })
        .collect();
      let etching = runestone.etching.map(|etching| {
        json!({
            "premine": etching.premine.map(|amount| amount.to_string()),
            "rune": etching.rune.map(|rune| rune.0.to_string()),
            "divisibility": etching.divisibility,
            "supply": etching.supply().map(|amount| amount.to_string()),
        })
      });
      json!({ "kind": "runestone", "edicts": edicts, "etching": etching,
          "pointer": runestone.pointer,
          "mint": runestone.mint.map(|id| id.to_string()),
      })
    }
  }
}

fn execute(request: &Value) -> Result<Value, String> {
  match field(request, "operation")? {
    "sat" => {
      let n = unsigned(request, "value")?;
      if n >= u128::from(Sat::SUPPLY) {
        return Err("sat is outside the Bitcoin supply".into());
      }
      let sat = Sat(n as u64);
      Ok(json!({ "n": sat.n().to_string(), "name": sat.name(),
          "height": sat.height().0, "rarity": sat.rarity().to_string(),
          "decimal": sat.decimal().to_string(), "degree": sat.degree().to_string(),
          "epoch": sat.epoch().0, "cycle": sat.cycle(), "charms": sat.charms(),
      }))
    }
    "varint_encode" => {
      let value = unsigned(request, "value")?;
      Ok(json!({ "hex": encode_hex(&varint::encode(value)) }))
    }
    "varint_decode" => {
      let bytes = decode_hex(field(request, "hex")?)?;
      let (value, consumed) = varint::decode(&bytes).map_err(|error| error.to_string())?;
      Ok(json!({ "value": value.to_string(), "consumed": consumed }))
    }
    "runestone_encode" => {
      let premine = unsigned(request, "premine")?;
      let amount = unsigned(request, "amount")?;
      let rune = unsigned(request, "rune")?;
      let mut etching = Etching {
        premine: Some(premine),
        rune: Some(Rune(rune)),
        ..Etching::default()
      };
      if request.get("cap").is_some() {
        etching.terms = Some(Terms {
          cap: Some(unsigned(request, "cap")?),
          amount: Some(unsigned(request, "term_amount")?),
          ..Terms::default()
        });
      }
      let runestone = Runestone {
        edicts: vec![Edict {
          id: RuneId {
            block: 840_000,
            tx: 1,
          },
          amount,
          output: 1,
        }],
        etching: Some(etching),
        pointer: Some(1),
        ..Runestone::default()
      };
      let script = runestone.encipher();
      let transaction = transaction_with_script(script.clone(), 2);
      Ok(json!({ "script_hex": encode_hex(script.as_bytes()),
                "artifact": describe_artifact(Runestone::decipher(&transaction)) }))
    }
    "runestone_script" => {
      let bytes = decode_hex(field(request, "hex")?)?;
      let count = request["output_count"].as_u64().unwrap_or(2);
      if !(1..=256).contains(&count) {
        return Err("output_count must be in 1..=256".into());
      }
      let transaction = transaction_with_script(ScriptBuf::from_bytes(bytes), count as usize);
      Ok(describe_artifact(Runestone::decipher(&transaction)))
    }
    "runestone_transaction" => {
      let bytes = decode_hex(field(request, "hex")?)?;
      let transaction: Transaction =
        consensus::deserialize(&bytes).map_err(|error| error.to_string())?;
      Ok(describe_artifact(Runestone::decipher(&transaction)))
    }
    _ => Err("unknown operation".into()),
  }
}

pub fn process(input: &[u8]) -> Vec<u8> {
  let result = if input.len() > MAX_INPUT_BYTES {
    Err("input exceeds 65536 bytes".into())
  } else {
    serde_json::from_slice::<Value>(input)
      .map_err(|error| error.to_string())
      .and_then(|request| execute(&request))
  };
  let response = match result {
    Ok(value) => json!({ "ok": value }),
    Err(error) => json!({ "error": error }),
  };
  serde_json::to_vec(&response).expect("response contains only serializable values")
}

#[unsafe(no_mangle)]
pub extern "C" fn ord_alloc(length: u32) -> u32 {
  if length == 0 || length as usize > MAX_INPUT_BYTES {
    return 0;
  }
  Box::into_raw(vec![0_u8; length as usize].into_boxed_slice()) as *mut u8 as u32
}

/// # Safety
/// We require an allocated live input region. The caller retains ownership.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ord_process(pointer: u32, length: u32) -> u64 {
  if pointer == 0 || length == 0 || length as usize > MAX_INPUT_BYTES {
    return 0;
  }
  let input = unsafe { std::slice::from_raw_parts(pointer as *const u8, length as usize) };
  let output = process(input).into_boxed_slice();
  let output_length = output.len() as u64;
  let output_pointer = Box::into_raw(output) as *mut u8 as u32;
  (output_length << 32) | u64::from(output_pointer)
}

/// # Safety
/// We release exactly one region returned by ord_alloc or ord_process, once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ord_free(pointer: u32, length: u32) {
  if pointer == 0 || length == 0 {
    return;
  }
  let slice = std::ptr::slice_from_raw_parts_mut(pointer as *mut u8, length as usize);
  unsafe {
    drop(Box::from_raw(slice));
  }
}
