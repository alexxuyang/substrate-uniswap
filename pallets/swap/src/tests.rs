use crate::{mock::*};
use frame_support::{assert_ok, traits::{OnFinalize, OnInitialize}};

fn run_to_block(n: u64) {
	while System::block_number() < n {
		Balances::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Balances::on_initialize(System::block_number());
	}
}

#[test]
fn run_to_block_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(System::block_number(), 0);
		run_to_block(10);
		assert_eq!(System::block_number(), 10);
	});
}

#[test]
fn swap_tests() {
	new_test_ext().execute_with(|| {
		run_to_block(1);

		let alice = 10u64;
		let bob = 20u64;

		assert_ok!(TokenModule::do_issue(alice, b"6666".to_vec(), 21000000));
		let token1_hash = TokenModule::owned_token((alice, 0)).unwrap();
		let token1 = TokenModule::token(token1_hash).unwrap();
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 21000000);

		assert_ok!(TokenModule::do_issue(alice, b"8888".to_vec(), 10000000));
		let token2_hash = TokenModule::owned_token((alice, 1)).unwrap();
		let token2 = TokenModule::token(token2_hash).unwrap();
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 10000000);

		assert_ok!(TokenModule::do_transfer(alice, bob, token1_hash, 1000000, None));
		assert_ok!(TokenModule::do_transfer(alice, bob, token2_hash, 5000000, None));
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 20000000);
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 5000000);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 1000000);
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 5000000);

		assert_ok!(SwapModule::do_create_trade_pair(alice, token1.hash, token2.hash));
		let tp_hash = SwapModule::trade_pair_hash_by_base_quote((token1.hash, token2.hash)).unwrap();
		let mut tp = SwapModule::trade_pair(tp_hash).unwrap();

		assert_eq!(tp.liquidity_token_issued_amount, 0);
		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value());
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 0);
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 0);
		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 0);

		// alice add liquidity
		assert_ok!(SwapModule::do_add_liquidity(alice, tp.hash, 100, Some(100 * 300))); // 100 eth & 30000 usdt
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 100);

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 100);
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999900); // 20000000 - 100
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 4970000);  // 1000000 - 100 * 300

		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 1000000);
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 5000000);

		// bob add liquidity
		assert_ok!(SwapModule::do_add_liquidity(bob, tp.hash, 500, None)); // 500 eth & 150000 usdt
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 100 + 500);

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 100);
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999900);
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 4970000);

		assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 999500);  // 1000000 - 500
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 4850000); // 5000000 - 500 * 300

		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value() - 600);
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 600);    // 100 + 500
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 180000); // 100 * 300 + 500 * 300

		// alice swap buy
		assert_ok!(SwapModule::do_swap_buy(alice, tp.hash, 13)); // 13 eth for 3818 usdt
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 100 + 500);

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 100);
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999887); // 19999900 - 13
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 4973818);  // 4970000 + 3818

		assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 999500);  // 1000000 - 500
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 4850000); // 5000000 - 500 * 300

		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value() - 600);
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 613);    // 600 + 13
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 176182); // 180000 - 3818

		// bob swap sell
		assert_ok!(SwapModule::do_swap_sell(bob, tp.hash, 539)); // 539 usdt for 2 eth
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 100 + 500);

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 100);
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999887); // 19999900 - 13
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 4973818);  // 4970000 + 3818

		assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 999502);  // 999500 + 2
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 4849461); // 4850000 - 539

		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value() - 600);
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 611);    // 613 - 2
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 176721); // 176182 + 539

		// alice add liquidity
		assert_ok!(SwapModule::do_add_liquidity(alice, tp.hash, 477, None)); // 477 eth & 137963 usdt
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 1068); // 600 + 468

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 568); // 100 + 468
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999410); // 19999887 - 477
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 4835855);  // 4973818 - 137963

		assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 999502);  // 999500 + 2
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 4849461); // 4850000 - 539

		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value() - 600 - 468);
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 1088);    // 611 + 477
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 314684);  // 176721 + 137963

		// bob remove liquidity
		assert_ok!(SwapModule::do_remove_liquidity(bob, tp.hash, 500)); // 509 eth & 147323 usdt
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 568); // 1068 - 500

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 568); // 100 + 468
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999410); // 19999887 - 477
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 4835855);  // 4973818 - 137963

		assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 0);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 1000011);  // 999502 + 509
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 4996784); // 4849461 + 147323

		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value() - 568);
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 579);    // 1088 - 509
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 167361);  // 314684 - 147323

		// alice remove liquidity
		assert_ok!(SwapModule::do_remove_liquidity(alice, tp.hash, 568)); // 579 eth & 167361 usdt
		tp = SwapModule::trade_pair(tp_hash).unwrap();
		assert_eq!(tp.liquidity_token_issued_amount, 0);

		assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 0); // 100 + 468
		assert_eq!(TokenModule::balance_of((alice, token1.hash)), 19999989); // 19999410 + 579
		assert_eq!(TokenModule::balance_of((alice, token2.hash)), 5003216);  // 4835855 + 167361

		assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 0);
		assert_eq!(TokenModule::balance_of((bob, token1.hash)), 1000011);  // 999502 + 509
		assert_eq!(TokenModule::balance_of((bob, token2.hash)), 4996784); // 4849461 + 147323

		assert_eq!(TokenModule::balance_of((tp.account, tp.liquidity_token_hash)), u128::max_value());
		assert_eq!(TokenModule::balance_of((tp.account, token1.hash)), 0);
		assert_eq!(TokenModule::balance_of((tp.account, token2.hash)), 0);
	});
}
