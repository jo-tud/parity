// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;

use ethcore::account_provider::AccountProvider;
use ethstore::EthStore;
use ethstore::dir::RootDiskDirectory;
use devtools::RandomTempPath;

use jsonrpc_core::IoHandler;
use v1::{ParityAccounts, ParityAccountsClient};

struct ParityAccountsTester {
	accounts: Arc<AccountProvider>,
	io: IoHandler,
}

fn accounts_provider() -> Arc<AccountProvider> {
	Arc::new(AccountProvider::transient_provider())
}

fn accounts_provider_with_vaults_support(temp_path: &str) -> Arc<AccountProvider> {
	let root_keys_dir = RootDiskDirectory::create(temp_path).unwrap();
	let secret_store = EthStore::open(Box::new(root_keys_dir)).unwrap();
	Arc::new(AccountProvider::new(Box::new(secret_store)))
}

fn setup_with_accounts_provider(accounts_provider: Arc<AccountProvider>) -> ParityAccountsTester {
	let parity_accounts = ParityAccountsClient::new(&accounts_provider);
	let mut io = IoHandler::default();
	io.extend_with(parity_accounts.to_delegate());

	let tester = ParityAccountsTester {
		accounts: accounts_provider,
		io: io,
	};

	tester
}

fn setup() -> ParityAccountsTester {
	setup_with_accounts_provider(accounts_provider())
}

fn setup_with_vaults_support(temp_path: &str) -> ParityAccountsTester {
	setup_with_accounts_provider(accounts_provider_with_vaults_support(temp_path))
}

#[test]
fn should_be_able_to_get_account_info() {
	let tester = setup();
	tester.accounts.new_account("").unwrap();
	let accounts = tester.accounts.accounts().unwrap();
	assert_eq!(accounts.len(), 1);
	let address = accounts[0];

	let uuid = tester.accounts.accounts_info().unwrap().get(&address).unwrap().uuid.as_ref().unwrap().clone();
	tester.accounts.set_account_name(address.clone(), "Test".to_owned()).unwrap();
	tester.accounts.set_account_meta(address.clone(), "{foo: 69}".to_owned()).unwrap();

	let request = r#"{"jsonrpc": "2.0", "method": "parity_allAccountsInfo", "params": [], "id": 1}"#;
	let res = tester.io.handle_request_sync(request);
	let response = format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"0x{}\":{{\"meta\":\"{{foo: 69}}\",\"name\":\"Test\",\"uuid\":\"{}\"}}}},\"id\":1}}", address.hex(), uuid);
	assert_eq!(res, Some(response));
}

#[test]
fn should_be_able_to_set_name() {
	let tester = setup();
	tester.accounts.new_account("").unwrap();
	let accounts = tester.accounts.accounts().unwrap();
	assert_eq!(accounts.len(), 1);
	let address = accounts[0];

	let request = format!(r#"{{"jsonrpc": "2.0", "method": "parity_setAccountName", "params": ["0x{}", "Test"], "id": 1}}"#, address.hex());
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	let res = tester.io.handle_request_sync(&request);
	assert_eq!(res, Some(response.into()));

	let uuid = tester.accounts.accounts_info().unwrap().get(&address).unwrap().uuid.as_ref().unwrap().clone();

	let request = r#"{"jsonrpc": "2.0", "method": "parity_allAccountsInfo", "params": [], "id": 1}"#;
	let res = tester.io.handle_request_sync(request);
	let response = format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"0x{}\":{{\"meta\":\"{{}}\",\"name\":\"Test\",\"uuid\":\"{}\"}}}},\"id\":1}}", address.hex(), uuid);
	assert_eq!(res, Some(response));
}

#[test]
fn should_be_able_to_set_meta() {
	let tester = setup();
	tester.accounts.new_account("").unwrap();
	let accounts = tester.accounts.accounts().unwrap();
	assert_eq!(accounts.len(), 1);
	let address = accounts[0];

	let request = format!(r#"{{"jsonrpc": "2.0", "method": "parity_setAccountMeta", "params": ["0x{}", "{{foo: 69}}"], "id": 1}}"#, address.hex());
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	let res = tester.io.handle_request_sync(&request);
	assert_eq!(res, Some(response.into()));

	let uuid = tester.accounts.accounts_info().unwrap().get(&address).unwrap().uuid.as_ref().unwrap().clone();

	let request = r#"{"jsonrpc": "2.0", "method": "parity_allAccountsInfo", "params": [], "id": 1}"#;
	let res = tester.io.handle_request_sync(request);
	let response = format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"0x{}\":{{\"meta\":\"{{foo: 69}}\",\"name\":\"\",\"uuid\":\"{}\"}}}},\"id\":1}}", address.hex(), uuid);
	assert_eq!(res, Some(response));
}

#[test]
fn rpc_parity_set_and_get_dapps_accounts() {
	// given
	let tester = setup();
	tester.accounts.set_address_name(10.into(), "10".into());
	assert_eq!(tester.accounts.dapps_addresses("app1".into()).unwrap(), vec![]);

	// when
	let request = r#"{"jsonrpc": "2.0", "method": "parity_setDappsAddresses","params":["app1",["0x000000000000000000000000000000000000000a","0x0000000000000000000000000000000000000001"]], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));

	// then
	assert_eq!(tester.accounts.dapps_addresses("app1".into()).unwrap(), vec![10.into()]);
	let request = r#"{"jsonrpc": "2.0", "method": "parity_getDappsAddresses","params":["app1"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":["0x000000000000000000000000000000000000000a"],"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_set_and_get_new_dapps_whitelist() {
	// given
	let tester = setup();

	// when set to whitelist
	let request = r#"{"jsonrpc": "2.0", "method": "parity_setNewDappsWhitelist","params":[["0x000000000000000000000000000000000000000a"]], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));

	// then
	assert_eq!(tester.accounts.new_dapps_whitelist().unwrap(), Some(vec![10.into()]));
	let request = r#"{"jsonrpc": "2.0", "method": "parity_getNewDappsWhitelist","params":[], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":["0x000000000000000000000000000000000000000a"],"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));

	// when set to empty
	let request = r#"{"jsonrpc": "2.0", "method": "parity_setNewDappsWhitelist","params":[null], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));

	// then
	assert_eq!(tester.accounts.new_dapps_whitelist().unwrap(), None);
	let request = r#"{"jsonrpc": "2.0", "method": "parity_getNewDappsWhitelist","params":[], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":null,"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_recent_dapps() {
	// given
	let tester = setup();

	// when
	// trigger dapp usage
	tester.accounts.note_dapp_used("dapp1".into()).unwrap();

	// then
	let request = r#"{"jsonrpc": "2.0", "method": "parity_listRecentDapps","params":[], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":{"dapp1":1},"id":1}"#;
	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn should_be_able_to_kill_account() {
	let tester = setup();
	tester.accounts.new_account("password").unwrap();
	let accounts = tester.accounts.accounts().unwrap();
	assert_eq!(accounts.len(), 1);
	let address = accounts[0];

	let request = format!(r#"{{"jsonrpc": "2.0", "method": "parity_killAccount", "params": ["0xf00baba2f00baba2f00baba2f00baba2f00baba2"], "id": 1}}"#);
	let response = r#"{"jsonrpc":"2.0","error":{"code":-32602,"message":"Invalid params","data":null},"id":1}"#;
	let res = tester.io.handle_request_sync(&request);
	assert_eq!(res, Some(response.into()));

	let request = format!(r#"{{"jsonrpc": "2.0", "method": "parity_killAccount", "params": ["0x{}", "password"], "id": 1}}"#, address.hex());
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	let res = tester.io.handle_request_sync(&request);
	assert_eq!(res, Some(response.into()));

	let accounts = tester.accounts.accounts().unwrap();
	assert_eq!(accounts.len(), 0);
}

#[test]
fn should_be_able_to_remove_address() {
	let tester = setup();

	// add an address
	let request = r#"{"jsonrpc": "2.0", "method": "parity_setAccountName", "params": ["0x000baba1000baba2000baba3000baba4000baba5", "Test"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;
	let res = tester.io.handle_request_sync(&request);
	assert_eq!(res, Some(response.into()));

	// verify it exists
	let request = r#"{"jsonrpc": "2.0", "method": "parity_allAccountsInfo", "params": [], "id": 2}"#;
	let res = tester.io.handle_request_sync(request);
	let response = r#"{"jsonrpc":"2.0","result":{"0x000baba1000baba2000baba3000baba4000baba5":{"meta":"{}","name":"Test"}},"id":2}"#;
	assert_eq!(res, Some(response.into()));

	// remove the address
	let request = r#"{"jsonrpc": "2.0", "method": "parity_removeAddress", "params": ["0x000baba1000baba2000baba3000baba4000baba5"], "id": 3}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":3}"#;
	let res = tester.io.handle_request_sync(&request);
	assert_eq!(res, Some(response.into()));

	// verify empty
	let request = r#"{"jsonrpc": "2.0", "method": "parity_allAccountsInfo", "params": [], "id": 4}"#;
	let res = tester.io.handle_request_sync(request);
	let response = r#"{"jsonrpc":"2.0","result":{},"id":4}"#;
	assert_eq!(res, Some(response.into()));
}

#[test]
fn rpc_parity_new_vault() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_newVault", "params":["vault1", "password1"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
	assert!(tester.accounts.close_vault("vault1").is_ok());
	assert!(tester.accounts.open_vault("vault1", "password1").is_ok());
}

#[test]
fn rpc_parity_open_vault() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());
	assert!(tester.accounts.close_vault("vault1").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_openVault", "params":["vault1", "password1"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_close_vault() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_closeVault", "params":["vault1"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_change_vault_password() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_changeVaultPassword", "params":["vault1", "password2"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_change_vault() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	let (address, _) = tester.accounts.new_account_and_public("root_password").unwrap();
	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());

	let request = format!(r#"{{"jsonrpc": "2.0", "method": "parity_changeVault", "params":["0x{}", "vault1"], "id": 1}}"#, address.hex());
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;

	assert_eq!(tester.io.handle_request_sync(&request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_vault_adds_vault_field_to_acount_meta() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	let (address1, _) = tester.accounts.new_account_and_public("root_password1").unwrap();
	let uuid1 = tester.accounts.account_meta(address1.clone()).unwrap().uuid.unwrap();
	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());
	assert!(tester.accounts.change_vault(address1, "vault1").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_allAccountsInfo", "params":[], "id": 1}"#;
	let response = format!(r#"{{"jsonrpc":"2.0","result":{{"0x{}":{{"meta":"{{\"vault\":\"vault1\"}}","name":"","uuid":"{}"}}}},"id":1}}"#, address1.hex(), uuid1);

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}

#[test]
fn rpc_parity_list_vaults() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());
	assert!(tester.accounts.create_vault("vault2", "password2").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_listVaults", "params":[], "id": 1}"#;
	let response1 = r#"{"jsonrpc":"2.0","result":["vault1","vault2"],"id":1}"#;
	let response2 = r#"{"jsonrpc":"2.0","result":["vault2","vault1"],"id":1}"#;

	let actual_response = tester.io.handle_request_sync(request);
	assert!(actual_response == Some(response1.to_owned())
		|| actual_response == Some(response2.to_owned()));
}

#[test]
fn rpc_parity_list_opened_vaults() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());
	assert!(tester.accounts.create_vault("vault2", "password2").is_ok());
	assert!(tester.accounts.create_vault("vault3", "password3").is_ok());
	assert!(tester.accounts.close_vault("vault2").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_listOpenedVaults", "params":[], "id": 1}"#;
	let response1 = r#"{"jsonrpc":"2.0","result":["vault1","vault3"],"id":1}"#;
	let response2 = r#"{"jsonrpc":"2.0","result":["vault3","vault1"],"id":1}"#;

	let actual_response = tester.io.handle_request_sync(request);
	assert!(actual_response == Some(response1.to_owned())
		|| actual_response == Some(response2.to_owned()));
}

#[test]
fn rpc_parity_get_set_vault_meta() {
	let temp_path = RandomTempPath::new();
	let tester = setup_with_vaults_support(temp_path.as_str());

	assert!(tester.accounts.create_vault("vault1", "password1").is_ok());
	assert!(tester.accounts.set_vault_meta("vault1", "vault1_meta").is_ok());

	let request = r#"{"jsonrpc": "2.0", "method": "parity_getVaultMeta", "params":["vault1"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":"vault1_meta","id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));

	let request = r#"{"jsonrpc": "2.0", "method": "parity_setVaultMeta", "params":["vault1", "updated_vault1_meta"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":true,"id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));

	let request = r#"{"jsonrpc": "2.0", "method": "parity_getVaultMeta", "params":["vault1"], "id": 1}"#;
	let response = r#"{"jsonrpc":"2.0","result":"updated_vault1_meta","id":1}"#;

	assert_eq!(tester.io.handle_request_sync(request), Some(response.to_owned()));
}
