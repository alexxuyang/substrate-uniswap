#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::result;
use sp_runtime::{traits::{Bounded, Member, Zero, CheckedSub, Hash, AtLeast32Bit}};
use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, Parameter,
					ensure, traits::{Get, Randomness}};
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
	lp_token_hash: T::Hash,
	account: T::AccountId,
}

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
	Buy, // give base, get quote
	Sell,
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
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
        /// Base equals to quote
		BaseEqualQuote,
        /// Token owner not found
        TokenOwnerNotFound,
        /// Sender not equal to base or quote owner
        SenderNotEqualToBaseOrQuoteOwner,
        /// Same trade pair with the given base and quote was already exist
        TradePairExisted,
        /// No matching trade pair found
        NoMatchingTradePair,
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
		pub fn add_liquidity(origin, hash: T::Hash, base_amount: T::Balance, quote_amount: T::Balance) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::do_add_liquidity(sender, hash, base_amount, quote_amount)
		}
	}
}

impl<T: Trait> Module<T> {
	fn do_create_trade_pair(sender: T::AccountId, base: T::Hash, quote: T::Hash) -> dispatch::DispatchResult {

		ensure!(base != quote, Error::<T>::BaseEqualQuote);

		let base_owner = <token::Module<T>>::owner(base);
		let quote_owner = <token::Module<T>>::owner(quote);

		ensure!(base_owner.is_some() && quote_owner.is_some(), Error::<T>::TokenOwnerNotFound);

		let base_owner = base_owner.unwrap();
		let quote_owner = quote_owner.unwrap();

		ensure!(sender == base_owner || sender == quote_owner, Error::<T>::SenderNotEqualToBaseOrQuoteOwner);

		let bq = Self::trade_pair_hash_by_base_quote((base, quote));
		let qb = Self::trade_pair_hash_by_base_quote((quote, base));

		ensure!(!bq.is_some() && !qb.is_some(), Error::<T>::TradePairExisted);

		let nonce = Nonce::get();

		let random_seed = <pallet_randomness_collective_flip::Module<T>>::random_seed();
		let hash = (random_seed, <frame_system::Module<T>>::block_number(), sender.clone(), base, quote, nonce)
			.using_encoded(<T as frame_system::Trait>::Hashing::hash);

		let account = Self::derivative_account_id(base, quote, hash);

		// todo: provide real symbol string
		let lp_token_hash = <token::Module<T>>::do_issue(sender.clone(), b"lp_token_hash".to_vec(), T::Balance::max_value())?;

		let tp = TradePair {
			hash, base, quote, account, lp_token_hash
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

	pub fn derivative_account_id(base: T::Hash, quote: T::Hash, hash: T::Hash) -> T::AccountId {
		let entropy = (b"substrate/uniswap", base, quote, hash).using_encoded(blake2_256);
		T::AccountId::decode(&mut &entropy[..]).unwrap_or_default()
	}

	fn do_add_liquidity(sender: T::AccountId, hash: T::Hash, base_amount: T::Balance, quote_amount: T::Balance) -> dispatch::DispatchResult {
		let tp = Self::trade_pair(hash).ok_or(Error::<T>::NoMatchingTradePair)?;



		Ok(())
	}
}
























