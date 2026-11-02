use bitcoin::{ScriptBuf, consensus, opcodes, script::Builder};
use ord_core_poc::{encode_hex, process, transaction_with_script};
use ordinals::{Sat, varint};
use serde_json::{Value, json};

fn add(vectors: &mut Vec<Value>, name: &str, request: Value) -> Value {
  let response: Value =
    serde_json::from_slice(&process(&serde_json::to_vec(&request).unwrap())).unwrap();
  vectors.push(json!({ "name": name, "request": request, "response": response }));
  response
}

fn script(integers: &[u128]) -> String {
  let mut payload = Vec::new();
  for integer in integers {
    varint::encode_to_vec(*integer, &mut payload);
  }
  let push = bitcoin::script::PushBytesBuf::try_from(payload).unwrap();
  encode_hex(
    Builder::new()
      .push_opcode(opcodes::all::OP_RETURN)
      .push_opcode(opcodes::all::OP_PUSHNUM_13)
      .push_slice(push)
      .into_script()
      .as_bytes(),
  )
}

fn main() {
  let mut vectors = Vec::new();
  for n in [
    0,
    1,
    4_999_999_999,
    5_000_000_000,
    1_050_000_000_000_000,
    Sat::LAST.0,
  ] {
    let response = add(
      &mut vectors,
      &format!("sat-{n}"),
      json!({ "operation": "sat", "value": n.to_string() }),
    );
    assert_eq!(response["ok"]["n"], n.to_string());
    if n == 0 {
      assert_eq!(response["ok"]["rarity"], "mythic");
      assert_eq!(response["ok"]["height"], 0);
    }
    if n == Sat::LAST.0 {
      assert_eq!(response["ok"]["name"], "a");
    }
  }
  for n in [Sat::SUPPLY, u64::MAX] {
    let response = add(
      &mut vectors,
      &format!("sat-invalid-{n}"),
      json!({ "operation": "sat", "value": n.to_string() }),
    );
    assert!(response.get("error").is_some());
  }
  let mut numbers = vec![
    0,
    1,
    127,
    128,
    255,
    u64::MAX as u128,
    1 << 64,
    1 << 127,
    u128::MAX,
  ];
  let mut state = 0xabc123u128;
  for _ in 0..128 {
    state = state
      .wrapping_mul(6364136223846793005)
      .wrapping_add(1442695040888963407);
    numbers.push(state);
  }
  for (index, number) in numbers.iter().enumerate() {
    let encoded = add(
      &mut vectors,
      &format!("varint-encode-{index}"),
      json!({ "operation": "varint_encode", "value": number.to_string() }),
    );
    let decoded = add(
      &mut vectors,
      &format!("varint-decode-{index}"),
      json!({ "operation": "varint_decode", "hex": encoded["ok"]["hex"] }),
    );
    assert_eq!(decoded["ok"]["value"], number.to_string());
  }
  for (name, hex) in [
    ("empty", "".to_owned()),
    ("unterminated", "80".to_owned()),
    ("overflow", "ff".repeat(18) + "04"),
    ("too-long", "80".repeat(20)),
    ("invalid-hex", "xz".to_owned()),
  ] {
    let response = add(
      &mut vectors,
      &format!("varint-invalid-{name}"),
      json!({ "operation": "varint_decode", "hex": hex }),
    );
    assert!(response.get("error").is_some());
  }
  let response = add(
    &mut vectors,
    "u128-overflow-string",
    json!({ "operation": "varint_encode", "value": "340282366920938463463374607431768211456" }),
  );
  assert!(response.get("error").is_some());
  let response = add(
    &mut vectors,
    "reject-imprecise-number",
    json!({ "operation": "varint_encode", "value": 9007199254740992u64 }),
  );
  assert!(response.get("error").is_some());

  for amount in [0, 1, 1 << 64, u128::MAX] {
    let response = add(
      &mut vectors,
      &format!("runestone-max-{amount}"),
      json!({ "operation": "runestone_encode", "premine": amount.to_string(), "amount": amount.to_string(), "rune": u128::MAX.to_string() }),
    );
    assert_eq!(response["ok"]["artifact"]["kind"], "runestone");
    assert_eq!(
      response["ok"]["artifact"]["etching"]["premine"],
      amount.to_string()
    );
    assert_eq!(
      response["ok"]["artifact"]["edicts"][0]["amount"],
      amount.to_string()
    );
    add(
      &mut vectors,
      &format!("runestone-roundtrip-{amount}"),
      json!({ "operation": "runestone_script", "hex": response["ok"]["script_hex"] }),
    );
  }
  let response = add(
    &mut vectors,
    "runestone-supply-overflow",
    json!({ "operation": "runestone_encode", "premine": u128::MAX.to_string(), "amount": "0", "rune": "0", "cap": "1", "term_amount": "1" }),
  );
  assert_eq!(response["ok"]["artifact"]["flaw"], "SupplyOverflow");

  let cases = [
    (
      "unrecognized-even-tag",
      script(&[126, 0]),
      "UnrecognizedEvenTag",
    ),
    ("truncated-field", script(&[22]), "TruncatedField"),
    ("trailing-integers", script(&[0, 1]), "TrailingIntegers"),
    (
      "invalid-output",
      script(&[0, 840000, 1, 1, 3]),
      "EdictOutput",
    ),
    ("invalid-rune-id", script(&[0, 0, 1, 1, 0]), "EdictRuneId"),
    ("unrecognized-flag", script(&[2, 128]), "UnrecognizedFlag"),
    ("malformed-varint", "6a5d0180".into(), "Varint"),
    ("invalid-script", "6a5d02ff".into(), "InvalidScript"),
    ("non-pushdata", "6a5d61".into(), "Opcode"),
  ];
  for (name, hex, flaw) in cases {
    let response = add(
      &mut vectors,
      name,
      json!({ "operation": "runestone_script", "hex": hex }),
    );
    assert_eq!(response["ok"]["kind"], "cenotaph");
    assert_eq!(response["ok"]["flaw"], flaw);
  }
  let response = add(
    &mut vectors,
    "no-runestone",
    json!({ "operation": "runestone_script", "hex": "6a" }),
  );
  assert_eq!(response["ok"]["kind"], "none");
  let response = add(
    &mut vectors,
    "empty-runestone",
    json!({ "operation": "runestone_script", "hex": "6a5d" }),
  );
  assert_eq!(response["ok"]["kind"], "runestone");

  let script = ScriptBuf::from_bytes(vec![0x6a, 0x5d]);
  let mut transaction = transaction_with_script(script, 2);
  transaction.input.push(bitcoin::TxIn::default());
  let response = add(
    &mut vectors,
    "wire-transaction",
    json!({ "operation": "runestone_transaction", "hex": encode_hex(&consensus::serialize(&transaction)) }),
  );
  assert_eq!(response["ok"]["kind"], "runestone");
  let response = add(
    &mut vectors,
    "malformed-transaction",
    json!({ "operation": "runestone_transaction", "hex": "00" }),
  );
  assert!(response.get("error").is_some());
  println!("{}", serde_json::to_string_pretty(&json!({ "upstream_revision": "1c2c8dee3694dbbfcdb45942257b2f1edda95bdc", "vector_count": vectors.len(), "vectors": vectors })).unwrap());
}
