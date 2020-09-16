#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{prelude::*};
use sp_runtime::{traits::{Bounded, Member, Zero, Hash, AtLeast32Bit}};
use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, Parameter,
					ensure, traits::{Randomness}};
use frame_system::ensure_signed;
use sp_io::hashing::blake2_256;

use codec::{Encode, Decode};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait Trait: token::Trait + frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Price: Parameter + Default + Member + Bounded + AtLeast32Bit + Copy + From<u128> + Into<u128>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TradePair<T> where T: Trait {
	hash: T::Hash,
	base: T::Hash,
	quote: T::Hash,
	liquidity_token_hash: T::Hash,
	liquidity_token_issued_amount: T::Balance,
	account: T::AccountId,
}

decl_storage! {
	trait Store for Module<T: Trait> as TemplateModule {
		///	TradePairHash => TradePair
		TradePairs get(fn trade_pair): map hasher(blake2_128_concat) T::Hash => Option<TradePair<T>>;
		/// (BaseTokenHash, quoteTokenHash) => TradePairHash
		TradePairsHashByBaseQuote get(fn trade_pair_hash_by_base_quote): map hasher(blake2_128_concat) (T::Hash, T::Hash) => Option<T::Hash>;
		/// Index => TradePairHash
		TradePairsHashByIndex get(fn trade_pair_hash_by_index): map hasher(blake2_128_concat) u64 => Option<T::Hash>;
		/// Index
		TradePairsIndex get(fn trade_pair_index): u64;

		Nonce: u64;
	}
}

decl_event!(
	pub enum Event<T>
	where
		<T as frame_system::Trait>::AccountId,
		<T as frame_system::Trait>::Hash,
		TradePair = TradePair<T>,
	{
		TradePairCreated(AccountId, Hash, TradePair),
		LiquidityAdded(AccountId, Hash),
		LiquidityRemoved(AccountId, Hash),
		SwapBuy(AccountId, Hash),
		SwapSell(AccountId, Hash),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
        /// Base equals to quote
		BaseEqualQuote,
        /// Token owner not found
        TokenOwnerNotFound,
        /// Token not found
        TokenNotFound,
        /// Sender not equal to base or quote owner
        SenderNotEqualToBaseOrQuoteOwner,
        /// Same trade pair with the given base and quote was already exist
        TradePairExisted,
        /// No matching trade pair found
        NoMatchingTradePair,
        /// Liquidity base & quote proportion is not same as pool's
        LiquidityProportionInvalid,
        /// Quote amount is none in init step of adding liquidity
        QuoteAmountIsNone,
        /// Base amount is zero
        BaseAmountIsZero,
        /// Qutoe amount is zero
        QuoteAmountIsZero,
        /// Liquidity minted amount is zero
        LiquidityMintedIsZero,
        /// 
        LiquidityTokenAmountOverflow,
        ///
        LiquidityTokenAmountIsZero,
        ///
        LiquidityTokenIssuedAmountIsZero,
        ///
        PoolBaseAmountIsZero,
        ///
        PoolQuoteAmountIsZero,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 10_000]
		pub fn create_trade_pair(origin, base: T::Hash, quote: T::Hash) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_create_trade_pair(sender, base, quote)
		}

		#[weight = 10_000]
		pub fn add_liquidity(origin, hash: T::Hash, base_amount: T::Balance, o_quote_amount: Option<T::Balance>) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_add_liquidity(sender, hash, base_amount, o_quote_amount)
		}

		#[weight = 10_000]
		pub fn remove_liquidity(origin, hash: T::Hash, liquidity_token_amount: T::Balance) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_remove_liquidity(sender, hash, liquidity_token_amount)
		}

		#[weight = 10_000]
		pub fn swap_buy(origin, hash: T::Hash, base_amount: T::Balance) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_swap_buy(sender, hash, base_amount)
		}

		#[weight = 10_000]
		pub fn swap_sell(origin, hash: T::Hash, quote_amount: T::Balance) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_swap_sell(sender, hash, quote_amount)
		}
	}
}

impl<T: Trait> Module<T> {
	fn do_create_trade_pair(sender: T::AccountId, base: T::Hash, quote: T::Hash) -> dispatch::DispatchResult {

		ensure!(base != quote, Error::<T>::BaseEqualQuote);

		let base_owner = <token::Module<T>>::owner(base).ok_or(Error::<T>::TokenOwnerNotFound)?;
		let quote_owner = <token::Module<T>>::owner(quote).ok_or(Error::<T>::TokenOwnerNotFound)?;
		ensure!(sender == base_owner || sender == quote_owner, Error::<T>::SenderNotEqualToBaseOrQuoteOwner);

		let base_token = <token::Module<T>>::token(base).ok_or(Error::<T>::TokenNotFound)?;
		let quote_token = <token::Module<T>>::token(base).ok_or(Error::<T>::TokenNotFound)?;

		let bq = Self::trade_pair_hash_by_base_quote((base, quote));
		let qb = Self::trade_pair_hash_by_base_quote((quote, base));

		ensure!(!bq.is_some() && !qb.is_some(), Error::<T>::TradePairExisted);

		let nonce = Nonce::get();

		let random_seed = <pallet_randomness_collective_flip::Module<T>>::random_seed();
		let hash = (random_seed, <frame_system::Module<T>>::block_number(), sender.clone(), base, quote, nonce)
			.using_encoded(<T as frame_system::Trait>::Hashing::hash);

		let account = Self::derivative_account_id(base, quote, hash);

		let mut lt_name = Vec::new();
		lt_name.extend(b"LT_".to_vec());
		lt_name.extend(base_token.symbol.clone());
		lt_name.extend(b"_".to_vec());
		lt_name.extend(quote_token.symbol.clone());

		let liquidity_token_hash = <token::Module<T>>::do_issue(account.clone(), lt_name, T::Balance::max_value())?;

		let tp: TradePair<T> = TradePair {
			hash, base, quote, account, liquidity_token_hash,
			liquidity_token_issued_amount: Zero::zero(),
		};

		Nonce::mutate(|n| *n += 1);
		TradePairs::insert(hash, tp.clone());
		TradePairsHashByBaseQuote::<T>::insert((base, quote), hash);

		let index = Self::trade_pair_index();
		TradePairsHashByIndex::<T>::insert(index, hash);
		TradePairsIndex::mutate(|n| *n += 1);

		Self::deposit_event(RawEvent::TradePairCreated(sender, hash, tp));

		Ok(())
	}

	fn derivative_account_id(base: T::Hash, quote: T::Hash, hash: T::Hash) -> T::AccountId {
		let entropy = (b"substrate/uniswap", base, quote, hash).using_encoded(blake2_256);
		T::AccountId::decode(&mut &entropy[..]).unwrap_or_default()
	}

	fn do_add_liquidity(sender: T::AccountId, hash: T::Hash, base_amount: T::Balance, o_quote_amount: Option<T::Balance>) -> dispatch::DispatchResult {
		let mut tp = Self::trade_pair(hash).ok_or(Error::<T>::NoMatchingTradePair)?;

		ensure!(base_amount > Zero::zero(), Error::<T>::BaseAmountIsZero);
		<token::Module<T>>::ensure_free_balance(sender.clone(), tp.base, base_amount)?;

		let pool_base_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.base));
		let pool_quote_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.quote));

		let quote_amount;
		let liquidity_minted;

		if pool_quote_amount == Zero::zero() || tp.liquidity_token_issued_amount == Zero::zero() { // init add liquidity
			ensure!(o_quote_amount.is_some(), Error::<T>::QuoteAmountIsNone);
			quote_amount = o_quote_amount.unwrap();
			liquidity_minted = base_amount;
		} else {
			// todo: overflow fix
			quote_amount = pool_quote_amount * base_amount / pool_base_amount;
			liquidity_minted = tp.liquidity_token_issued_amount * base_amount / pool_base_amount;
		}

		ensure!(quote_amount > Zero::zero(), Error::<T>::QuoteAmountIsZero);
		ensure!(liquidity_minted > Zero::zero(), Error::<T>::LiquidityMintedIsZero);
		// ensure!(pool_base_amount * quote_amount == pool_quote_amount * base_amount, Error::<T>::LiquidityProportionInvalid);

		<token::Module<T>>::ensure_free_balance(sender.clone(), tp.quote, quote_amount)?;
		<token::Module<T>>::ensure_free_balance(tp.account.clone(), tp.liquidity_token_hash, liquidity_minted)?;

		<token::Module<T>>::do_transfer(sender.clone(), tp.account.clone(), tp.base, base_amount, None)?;
		<token::Module<T>>::do_transfer(sender.clone(), tp.account.clone(), tp.quote, quote_amount, None)?;
		<token::Module<T>>::do_transfer(tp.account.clone(), sender.clone(), tp.liquidity_token_hash, liquidity_minted, None)?;

		tp.liquidity_token_issued_amount = tp.liquidity_token_issued_amount + liquidity_minted;
		<TradePairs<T>>::insert(hash, tp);

		Self::deposit_event(RawEvent::LiquidityAdded(sender, hash));

		Ok(())
	}

	fn do_remove_liquidity(sender: T::AccountId, hash: T::Hash, liquidity_token_amount: T::Balance) -> dispatch::DispatchResult {
		
		let mut tp = Self::trade_pair(hash).ok_or(Error::<T>::NoMatchingTradePair)?;

		ensure!(liquidity_token_amount <= tp.liquidity_token_issued_amount, Error::<T>::LiquidityTokenAmountOverflow);

		ensure!(liquidity_token_amount > Zero::zero(), Error::<T>::LiquidityTokenAmountIsZero);		
		ensure!(tp.liquidity_token_issued_amount > Zero::zero(), Error::<T>::LiquidityTokenIssuedAmountIsZero);		

		let pool_base_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.base));
		let pool_quote_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.quote));
		ensure!(pool_base_amount > Zero::zero(), Error::<T>::PoolBaseAmountIsZero);
		ensure!(pool_quote_amount > Zero::zero(), Error::<T>::PoolQuoteAmountIsZero);

		let base_amount = pool_base_amount * liquidity_token_amount / tp.liquidity_token_issued_amount;
		let quote_amount = pool_quote_amount * liquidity_token_amount / tp.liquidity_token_issued_amount;
		ensure!(quote_amount > Zero::zero(), Error::<T>::QuoteAmountIsZero);
		ensure!(base_amount > Zero::zero(), Error::<T>::BaseAmountIsZero);

		<token::Module<T>>::ensure_free_balance(tp.account.clone(), tp.base, base_amount)?;
		<token::Module<T>>::ensure_free_balance(tp.account.clone(), tp.quote, quote_amount)?;
		<token::Module<T>>::ensure_free_balance(sender.clone(), tp.liquidity_token_hash, liquidity_token_amount)?;

		<token::Module<T>>::do_transfer(tp.account.clone(), sender.clone(), tp.base, base_amount, None)?;
		<token::Module<T>>::do_transfer(tp.account.clone(), sender.clone(), tp.quote, quote_amount, None)?;
		<token::Module<T>>::do_transfer(sender.clone(), tp.account.clone(), tp.liquidity_token_hash, liquidity_token_amount, None)?;

		tp.liquidity_token_issued_amount = tp.liquidity_token_issued_amount - liquidity_token_amount;
		<TradePairs<T>>::insert(hash, tp);

		Self::deposit_event(RawEvent::LiquidityRemoved(sender, hash));

		Ok(())
	}

	fn do_swap_buy(sender: T::AccountId, hash: T::Hash, base_amount: T::Balance) -> dispatch::DispatchResult {

		let tp = Self::trade_pair(hash).ok_or(Error::<T>::NoMatchingTradePair)?;

		let pool_base_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.base));
		let pool_quote_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.quote));
		ensure!(pool_base_amount > Zero::zero(), Error::<T>::PoolBaseAmountIsZero);
		ensure!(pool_quote_amount > Zero::zero(), Error::<T>::PoolQuoteAmountIsZero);

		// todo: add fee support
		let quote_amount = (pool_quote_amount * (pool_base_amount + base_amount) - pool_quote_amount * pool_base_amount)
							/ (pool_base_amount + base_amount);

		ensure!(quote_amount > Zero::zero(), Error::<T>::QuoteAmountIsZero);
		ensure!(base_amount > Zero::zero(), Error::<T>::BaseAmountIsZero);

		<token::Module<T>>::ensure_free_balance(sender.clone(), tp.base, base_amount)?;
		<token::Module<T>>::ensure_free_balance(tp.account.clone(), tp.quote, quote_amount)?;

		<token::Module<T>>::do_transfer(sender.clone(), tp.account.clone(), tp.base, base_amount, None)?;
		<token::Module<T>>::do_transfer(tp.account.clone(), sender.clone(), tp.quote, quote_amount, None)?;

		Self::deposit_event(RawEvent::SwapBuy(sender, hash));

		Ok(())
	}

	fn do_swap_sell(sender: T::AccountId, hash: T::Hash, quote_amount: T::Balance) -> dispatch::DispatchResult {

		let tp = Self::trade_pair(hash).ok_or(Error::<T>::NoMatchingTradePair)?;

		let pool_base_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.base));
		let pool_quote_amount = <token::Module<T>>::balance_of((tp.account.clone(), tp.quote));
		ensure!(pool_base_amount > Zero::zero(), Error::<T>::PoolBaseAmountIsZero);
		ensure!(pool_quote_amount > Zero::zero(), Error::<T>::PoolQuoteAmountIsZero);

		let base_amount = (pool_base_amount * (pool_quote_amount + quote_amount) - pool_quote_amount * pool_base_amount)
							/ (pool_quote_amount + quote_amount);

		ensure!(quote_amount > Zero::zero(), Error::<T>::QuoteAmountIsZero);
		ensure!(base_amount > Zero::zero(), Error::<T>::BaseAmountIsZero);

		<token::Module<T>>::ensure_free_balance(tp.account.clone(), tp.base, base_amount)?;
		<token::Module<T>>::ensure_free_balance(sender.clone(), tp.quote, quote_amount)?;

		<token::Module<T>>::do_transfer(tp.account.clone(), sender.clone(), tp.base, base_amount, None)?;
		<token::Module<T>>::do_transfer(sender.clone(), tp.account.clone(), tp.quote, quote_amount, None)?;

		Self::deposit_event(RawEvent::SwapSell(sender, hash));

		Ok(())
	}
}























