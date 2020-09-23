use crate::mock::*;
use frame_support::{
    assert_ok,
    traits::{OnFinalize, OnInitialize},
};
use token::*;

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

        assert_ok!(TokenModule::do_issue(
            alice,
            b"6666".to_vec(),
            21000000,
            TokenType::Normal
        ));
        let token1_hash = TokenModule::token_hash_by_index(0).unwrap();
        let token1 = TokenModule::token(token1_hash).unwrap();
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            21000000
        );

        assert_ok!(TokenModule::do_issue(
            alice,
            b"8888".to_vec(),
            10000000,
            TokenType::Normal
        ));
        let token2_hash = TokenModule::token_hash_by_index(1).unwrap();
        let token2 = TokenModule::token(token2_hash).unwrap();
        assert_eq!(
            TokenModule::balance_of((alice, token2.token_hash)),
            10000000
        );

        assert_ok!(TokenModule::do_transfer(
            alice,
            bob,
            token1_hash,
            1000000,
            None
        ));
        assert_ok!(TokenModule::do_transfer(
            alice,
            bob,
            token2_hash,
            5000000,
            None
        ));
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            20000000
        );
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 5000000);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 1000000);
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 5000000);

        assert_ok!(SwapModule::do_create_trade_pair(
            alice,
            token1.token_hash,
            token2.token_hash
        ));
        let tp_hash =
            SwapModule::trade_pair_hash_by_base_quote((token1.token_hash, token2.token_hash))
                .unwrap();
        let mut tp = SwapModule::trade_pair(tp_hash).unwrap();

        assert_eq!(tp.liquidity_token_issued_amount, 0);
        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value()
        );
        assert_eq!(TokenModule::balance_of((tp.account, token1.token_hash)), 0);
        assert_eq!(TokenModule::balance_of((tp.account, token2.token_hash)), 0);
        assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 0);

        // alice add liquidity
        assert_ok!(SwapModule::do_add_liquidity(
            alice,
            tp.tp_hash,
            100,
            Some(100 * 300)
        )); // 100 eth & 30000 usdt
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 100);

        assert_eq!(
            TokenModule::balance_of((alice, tp.liquidity_token_hash)),
            100
        );
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999900
        ); // 20000000 - 100
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 4970000); // 1000000 - 100 * 300

        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 1000000);
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 5000000);

        // bob add liquidity
        assert_ok!(SwapModule::do_add_liquidity(bob, tp.tp_hash, 500, None)); // 500 eth & 150000 usdt
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 100 + 500);

        assert_eq!(
            TokenModule::balance_of((alice, tp.liquidity_token_hash)),
            100
        );
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999900
        );
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 4970000);

        assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 999500); // 1000000 - 500
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 4850000); // 5000000 - 500 * 300

        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value() - 600
        );
        assert_eq!(
            TokenModule::balance_of((tp.account, token1.token_hash)),
            600
        ); // 100 + 500
        assert_eq!(
            TokenModule::balance_of((tp.account, token2.token_hash)),
            180000
        ); // 100 * 300 + 500 * 300

        // alice swap buy
        assert_ok!(SwapModule::do_swap_buy(alice, tp.tp_hash, 13)); // 13 eth for 3817 usdt
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 100 + 500);

        assert_eq!(
            TokenModule::balance_of((alice, tp.liquidity_token_hash)),
            100
        );
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999887
        ); // 19999900 - 13
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 4973817); // 4970000 + 3817

        assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 999500); // 1000000 - 500
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 4850000); // 5000000 - 500 * 300

        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value() - 600
        );
        assert_eq!(
            TokenModule::balance_of((tp.account, token1.token_hash)),
            613
        ); // 600 + 13
        assert_eq!(
            TokenModule::balance_of((tp.account, token2.token_hash)),
            176183
        ); // 180000 - 3817

        // bob swap sell
        assert_ok!(SwapModule::do_swap_sell(bob, tp.tp_hash, 539)); // 539 usdt for 1 eth
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 100 + 500);

        assert_eq!(
            TokenModule::balance_of((alice, tp.liquidity_token_hash)),
            100
        );
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999887
        ); // 19999900 - 13
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 4973817); // 4970000 + 3817

        assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 999501); // 999500 + 1
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 4849461); // 4850000 - 539

        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value() - 600
        );
        assert_eq!(
            TokenModule::balance_of((tp.account, token1.token_hash)),
            612
        ); // 613 - 1
        assert_eq!(
            TokenModule::balance_of((tp.account, token2.token_hash)),
            176722
        ); // 176183 + 539

        // alice add liquidity
        assert_ok!(SwapModule::do_add_liquidity(alice, tp.tp_hash, 477, None)); // 477 eth & 137739 usdt
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 1067); // 600 + 467

        assert_eq!(
            TokenModule::balance_of((alice, tp.liquidity_token_hash)),
            567
        ); // 100 + 467
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999410
        ); // 19999887 - 477
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 4836078); // 4973817 - 137739

        assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 500);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 999501); // 999500 + 1
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 4849461); // 4850000 - 539

        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value() - 600 - 467
        );
        assert_eq!(
            TokenModule::balance_of((tp.account, token1.token_hash)),
            1089
        ); // 612 + 477
        assert_eq!(
            TokenModule::balance_of((tp.account, token2.token_hash)),
            314461
        ); // 176722 + 137739

        // bob remove liquidity
        assert_ok!(SwapModule::do_remove_liquidity(bob, tp.tp_hash, 500)); // 510 eth & 147357 usdt
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 567); // 1067 - 500

        assert_eq!(
            TokenModule::balance_of((alice, tp.liquidity_token_hash)),
            567
        ); // 100 + 467
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999410
        ); // 19999887 - 477
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 4836078); // 4973817 - 137739

        assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 0);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 1000011); // 999501 + 510
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 4996818); // 4849461 + 147357

        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value() - 567
        );
        assert_eq!(
            TokenModule::balance_of((tp.account, token1.token_hash)),
            579
        ); // 1089 - 510
        assert_eq!(
            TokenModule::balance_of((tp.account, token2.token_hash)),
            167104
        ); // 314461 - 147357

        // alice remove liquidity
        assert_ok!(SwapModule::do_remove_liquidity(alice, tp.tp_hash, 567)); // 579 eth & 167104 usdt
        tp = SwapModule::trade_pair(tp_hash).unwrap();
        assert_eq!(tp.liquidity_token_issued_amount, 0);

        assert_eq!(TokenModule::balance_of((alice, tp.liquidity_token_hash)), 0);
        assert_eq!(
            TokenModule::balance_of((alice, token1.token_hash)),
            19999989
        ); // 19999410 + 579
        assert_eq!(TokenModule::balance_of((alice, token2.token_hash)), 5003182); // 4836078 + 167104

        assert_eq!(TokenModule::balance_of((bob, tp.liquidity_token_hash)), 0);
        assert_eq!(TokenModule::balance_of((bob, token1.token_hash)), 1000011); // 999501 + 510
        assert_eq!(TokenModule::balance_of((bob, token2.token_hash)), 4996818); // 4849461 + 147357

        assert_eq!(
            TokenModule::balance_of((tp.account, tp.liquidity_token_hash)),
            u128::max_value()
        );
        assert_eq!(TokenModule::balance_of((tp.account, token1.token_hash)), 0);
        assert_eq!(TokenModule::balance_of((tp.account, token2.token_hash)), 0);
    });
}
