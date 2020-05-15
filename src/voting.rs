use crate::crypto::{SdkClient, parse_string};
use crate::config::Config;
use crate::call;
use serde_json;
use ton_client_rs::{
    TonClient
};

const MSIG_ABI: &str = r#"{
	"ABI version": 2,
	"header": ["pubkey", "time", "expire"],
	"functions": [
		{
			"name": "constructor",
			"inputs": [
				{"name":"owners","type":"uint256[]"},
				{"name":"reqConfirms","type":"uint8"}
			],
			"outputs": [
			]
		},
		{
			"name": "acceptTransfer",
			"inputs": [
				{"name":"payload","type":"bytes"}
			],
			"outputs": [
			]
		},
		{
			"name": "sendTransaction",
			"inputs": [
				{"name":"dest","type":"address"},
				{"name":"value","type":"uint128"},
				{"name":"bounce","type":"bool"},
				{"name":"flags","type":"uint8"},
				{"name":"payload","type":"cell"}
			],
			"outputs": [
			]
		},
		{
			"name": "submitTransaction",
			"inputs": [
				{"name":"dest","type":"address"},
				{"name":"value","type":"uint128"},
				{"name":"bounce","type":"bool"},
				{"name":"allBalance","type":"bool"},
				{"name":"payload","type":"cell"}
			],
			"outputs": [
				{"name":"transId","type":"uint64"}
			]
		},
		{
			"name": "confirmTransaction",
			"inputs": [
				{"name":"transactionId","type":"uint64"}
			],
			"outputs": [
			]
		},
		{
			"name": "isConfirmed",
			"inputs": [
				{"name":"mask","type":"uint32"},
				{"name":"index","type":"uint8"}
			],
			"outputs": [
				{"name":"confirmed","type":"bool"}
			]
		},
		{
			"name": "getParameters",
			"inputs": [
			],
			"outputs": [
				{"name":"maxQueuedTransactions","type":"uint8"},
				{"name":"maxCustodianCount","type":"uint8"},
				{"name":"expirationTime","type":"uint64"},
				{"name":"minValue","type":"uint128"},
				{"name":"requiredTxnConfirms","type":"uint8"}
			]
		},
		{
			"name": "getTransaction",
			"inputs": [
				{"name":"transactionId","type":"uint64"}
			],
			"outputs": [
				{"components":[{"name":"id","type":"uint64"},{"name":"confirmationsMask","type":"uint32"},{"name":"signsRequired","type":"uint8"},{"name":"signsReceived","type":"uint8"},{"name":"creator","type":"uint256"},{"name":"index","type":"uint8"},{"name":"dest","type":"address"},{"name":"value","type":"uint128"},{"name":"sendFlags","type":"uint16"},{"name":"payload","type":"cell"},{"name":"bounce","type":"bool"}],"name":"trans","type":"tuple"}
			]
		},
		{
			"name": "getTransactions",
			"inputs": [
			],
			"outputs": [
				{"components":[{"name":"id","type":"uint64"},{"name":"confirmationsMask","type":"uint32"},{"name":"signsRequired","type":"uint8"},{"name":"signsReceived","type":"uint8"},{"name":"creator","type":"uint256"},{"name":"index","type":"uint8"},{"name":"dest","type":"address"},{"name":"value","type":"uint128"},{"name":"sendFlags","type":"uint16"},{"name":"payload","type":"cell"},{"name":"bounce","type":"bool"}],"name":"transactions","type":"tuple[]"}
			]
		},
		{
			"name": "getTransactionIds",
			"inputs": [
			],
			"outputs": [
				{"name":"ids","type":"uint64[]"}
			]
		},
		{
			"name": "getCustodians",
			"inputs": [
			],
			"outputs": [
				{"components":[{"name":"index","type":"uint8"},{"name":"pubkey","type":"uint256"}],"name":"custodians","type":"tuple[]"}
			]
		}
	],
	"data": [
	],
	"events": [
		{
			"name": "TransferAccepted",
			"inputs": [
				{"name":"payload","type":"bytes"}
			],
			"outputs": [
			]
		}
	]
}"#;

const TRANSFER_WITH_COMMENT: &str = r#"{
	"ABI version": 1,
	"functions": [
		{
			"name": "transfer",
			"id": "0x00000000",
			"inputs": [{"name":"comment","type":"bytes"}],
			"outputs": []
		},
	],
	"events": [],
	"data": []
};
"#;

fn encode_transfer_body(text: &str) -> Result<String, String> {
	let text = hex::encode(text.as_bytes());

	let client = SdkClient::new();

    parse_string(client.request(
        "contracts.run.body",
        json!({
            "abi": json!(TRANSFER_WITH_COMMENT),
            "function": "transfer",
            "params": json!({
				"comment": text
			}),
			"internal": true,
        })
    )?)
}

pub fn create_proposal(
	conf: Config,
	addr: &str,
	keys: Option<String>,
	dest: &str,
	text: &str,
) -> Result<(), String> {

	let msg_body = encode_transfer_body(text)?;
	
	let params = json!({
		"dest": dest,
		"value": 1000000,
		"bounce": true,
		"allBalance": false,
		"payload": msg_body,
	});

 	call::call_contract(
		conf,
		addr,
		MSIG_ABI.to_string(),
		"submitTransaction",
		&serde_json::to_string(&params).unwrap(),
		keys,
		false
	)
}

pub fn vote(
	conf: Config,
	addr: &str,
	keys: Option<String>,
	trid: &str,
) -> Result<(), String> {

	let params = json!({
		"transactionId": trid,
	});

 	call::call_contract(
		conf,
		addr,
		MSIG_ABI.to_string(),
		"confirmTransaction",
		&serde_json::to_string(&params).unwrap(),
		keys,
		false
	)
}

pub fn decode_proposal(
	conf: Config,
	addr: &str,
	trid: &str,
) -> Result<(), String> {

	let result = call::call_contract_with_result(
		conf,
		addr,
		MSIG_ABI.to_string(),
		"getTransactions",
		"{}",
		None,
		true
	)?;

	let txns = result["output"]["transactions"].as_array().unwrap();

	for txn in txns {
		if txn["id"].as_str().unwrap() == trid {
			let body = txn["body"].as_str().unwrap();
			let ton = TonClient::default()
				.map_err(|e| format!("failed to create tonclient: {}", e.to_string()))?;
			
			let result = ton.contracts.decode_input_message_body(
				TRANSFER_WITH_COMMENT,
				&base64::decode(&body).unwrap()
			).map_err(|e| format!("transaction doesn't contain comment: {}", e))?;
	
			let comment = String::from_utf8(hex::decode(result.output["comment"].as_str().unwrap()).unwrap()).unwrap();
	
			println!("Proposal Comment: {}", comment);
			return Ok(());
		}
	}
	println!("Proposal with id {} not found", trid);
	Ok(())
}