#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedSub};
	use frame_support::sp_runtime::SaturatedConversion;
	use frame_support::{
		inherent::Vec,
		pallet_prelude::{CountedStorageMap, *},
		traits::{Currency, LockableCurrency, ReservableCurrency},
		Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;
	use integer_sqrt::IntegerSquareRoot;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		/// Voting period in blocks.
		type VotingPeriod: Get<Self::BlockNumber>;
	}
	// I use some type alias to make the code more readable (I also use this types on my tests)
	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub type ProposalIndex = u32;

	#[pallet::storage]
	pub type RegisteredVoters<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, bool, OptionQuery>;

	#[pallet::storage]
	pub type Proposals<T: Config> =
		CountedStorageMap<_, Blake2_128Concat, ProposalIndex, Proposal<T>, OptionQuery>;

	#[pallet::storage]
	pub type AyeVotes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProposalIndex,
		Blake2_128Concat,
		AccountIdOf<T>,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Clone, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct Proposal<T: Config> {
		proposal_index: u32,
		text: T::Hash,
		proposer: AccountIdOf<T>,
		end_block: T::BlockNumber,
		status: ProposalStatus,
	}

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Clone, PartialEq)]
	pub enum Vote {
		Aye,
		Nay,
		Abstain,
	}

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Clone, PartialEq)]
	pub enum ProposalStatus {
		NotStarted,
		InProgress,
		Completed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New voter registered. [who]
		VoterRegistered { voter_id: AccountIdOf<T>, initial_balance: BalanceOf<T> },
		/// New proposal created. [proposal_index, text, end_block]
		NewProposalCreated {
			proposal_index: ProposalIndex,
			text: T::Hash,
			end_block: T::BlockNumber,
		},
		/// Reserved tokens for a proposal. [who, amount]
		TokensReserved { who: AccountIdOf<T>, amount: BalanceOf<T> },
		/// Proposal started. [proposal_index]
		ProposalStarted { proposal_index: ProposalIndex },
		/// Voted for a proposal. [proposal_index, vote]
		ProposalVoted { proposal_index: ProposalIndex, vote: Vote },
		/// Proposals Voted
		ProposalsVoted { proposals: Vec<ProposalIndex> },
		/// Unreserve tokens. [who, amount, updated_balance]
		TokensUnreserved {
			who: AccountIdOf<T>,
			amount: BalanceOf<T>,
			updated_balance: BalanceOf<T>,
		},
		/// Voting ended.[proposal_index]
		VotingEnded { winner: ProposalIndex },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Not a registered voter
		NotRegisteredVoter,
		/// Proposal not found
		ProposalNotFound,
		/// Proposal is not active
		ProposalNotActive,
		/// Proposal already started
		ProposalAlreadyStarted,
		/// Not enough balance to reserve
		NotEnoughBalance,
		/// Not enough reserved tokens
		NotReservedTokens,
		/// Voter already voted
		VoterAlreadyVoted,
		/// Not enough reserved tokens
		NotEnoughReservedTokens,
		/// Insufficient fee
		InsufficientFee,
		/// Voter already registered
		VoterAlreadyRegistered,
		/// Invalid tokens amount to reserve
		InvalidTokensAmountToReserve,
		/// At least one of the proposals given is not registered or not active
		AtLeastOneProposalNotRegisteredOrNotActive,
		/// Invalid tokens amount to unreserve
		InvalidTokensAmountToUnreserve,
		/// Balance Overflow
		SubstractionOverflow,
		/// Slash failed
		SlashFailed,
		/// Balance addition overflow
		AdditionOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/*
			* Register a new voter
			 * @param fee: Fee to register a new voter
			* @return DispatchResult

			* This function will create a new voter and will reserve 100 - fee tokens to be used as voting tokens
			* To create a new user, "root user" must call this function passing the user id and the fee

		*/
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn register_voter(
			origin: OriginFor<T>,
			voter_id: AccountIdOf<T>,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(!Self::is_voter_registered(&voter_id), Error::<T>::VoterAlreadyRegistered);
			ensure!(fee > 0u32.into(), Error::<T>::InsufficientFee);
			// Create initial balance for the voter equals to 100 - fee
			let initial_balance_without_fee: BalanceOf<T> = 100u32.into();
			ensure!(
				Self::checked_sub_between_balances(initial_balance_without_fee, fee).is_ok(),
				Error::<T>::SubstractionOverflow
			);

			let initial_balance =
				Self::checked_sub_between_balances(initial_balance_without_fee, fee)?;
			T::Currency::make_free_balance_be(&voter_id, initial_balance);

			RegisteredVoters::<T>::insert(&voter_id, true);
			Self::deposit_event(Event::VoterRegistered { voter_id, initial_balance });
			Ok(())
		}

		/*
			* Create a new proposal
			 * @param text: Proposal text
			* @return DispatchResult

			* This function will create a new proposal.
			* The only requirement is that the user must be a registered voter.
			* To create a new proposal, a registered voter must call this function passing the proposal text.

		*/
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn create_proposal(origin: OriginFor<T>, text: T::Hash) -> DispatchResult {
			let proposer = ensure_signed(origin)?;
			ensure!(Self::is_voter_registered(&proposer), Error::<T>::NotRegisteredVoter);

			let proposal_index = Proposals::<T>::count() + 1;

			let end_block = <frame_system::Pallet<T>>::block_number() + T::VotingPeriod::get();

			let proposal = Proposal {
				proposal_index,
				text,
				proposer,
				end_block,
				status: ProposalStatus::NotStarted,
			};

			Proposals::<T>::insert(proposal_index, proposal);
			Self::deposit_event(Event::NewProposalCreated { proposal_index, text, end_block });
			Ok(())
		}

		/*
			* Start a proposal
			 * @param proposal_index: Proposal index to start
			* @param fee: Fee to start a proposal
			* @return DispatchResult

			* This function will start a proposal.
			* The only requirement is that the user must be a registered voter
			* Ii will check if the proposal exists and if the proposal is not already started
			* To start a proposal, a registered voter must call this function passing the proposal index and the fee needed to start the proposal
		*/
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn start_proposal(
			origin: OriginFor<T>,
			proposal_index: ProposalIndex,
			fee: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(Self::is_voter_registered(&who), Error::<T>::NotRegisteredVoter);
			ensure!(Self::get_proposal(proposal_index).is_some(), Error::<T>::ProposalNotFound);
			let balance = Self::get_voter_balance(&who);
			ensure!(balance >= fee, Error::<T>::NotEnoughBalance);
			ensure!(
				Self::get_proposal_status(proposal_index) == ProposalStatus::NotStarted,
				Error::<T>::ProposalAlreadyStarted
			);
			ensure!(fee > 0u32.into(), Error::<T>::InsufficientFee);

			let proposal: Proposal<T> =
				Self::get_proposal(proposal_index).ok_or("Proposal not found")?;

			let proposal_updated: Proposal<T> = Proposal {
				proposal_index,
				text: proposal.text,
				proposer: proposal.proposer,
				end_block: proposal.end_block,
				status: ProposalStatus::InProgress,
			};

			Proposals::<T>::mutate(proposal_index, |p| *p = Some(proposal_updated));

			T::Currency::make_free_balance_be(&who, balance - fee);
			Self::deposit_event(Event::ProposalStarted { proposal_index });

			Ok(())
		}

		/*
			* Reserve tokens
			 * @param amount: Amount of tokens to reserve
			* @return DispatchResult

			* This function will reserve tokens.
			* The requirements are:
				- The user must be a registered voter
				- The amount of tokens to reserve must be greater than 0
				- The user must have enough balance to reserve the tokens
			* To reserve tokens, a registered voter must call this function passing the amount of tokens to reserve
		*/
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn reserve_tokens(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_voter_registered(&who), Error::<T>::NotRegisteredVoter);
			ensure!(amount > 0u32.into(), Error::<T>::InvalidTokensAmountToReserve);

			let voter_balance = Self::get_voter_balance(&who);
			ensure!(voter_balance >= amount, Error::<T>::NotEnoughBalance);

			// Reserve tokens
			T::Currency::reserve(&who, amount)?;
			Self::deposit_event(Event::TokensReserved { who, amount });

			Ok(())
		}

		/*
			* Vote a proposal
			 * @param proposal_index: Proposal index
			* @param vote: Vote
			* @return DispatchResult

			* This function will vote a proposal.
			* The requirements are:
				- The user must be a registered voter
				- The proposal must be registered
				- The proposal must be active
				- The voting period must be still live
			* To vote a proposal, a registered voter must call this function passing the proposal index and the vote
		*/
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn vote_proposal(
			origin: OriginFor<T>,
			proposal_index: u32,
			vote: Vote,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_voter_registered(&who), Error::<T>::NotRegisteredVoter);
			ensure!(Self::is_proposal_registered(proposal_index), Error::<T>::ProposalNotFound);
			ensure!(Self::is_proposal_active(proposal_index), Error::<T>::ProposalNotActive);

			// Check if the VotingEnded is still live
			let current_block = <frame_system::Pallet<T>>::block_number();
			let proposal_end_block = Self::get_proposal_end_block(proposal_index);

			if proposal_end_block <= current_block {
				Self::update_proposal_status_to_completed(proposal_index);

				let winner = Self::get_winner();
				Self::deposit_event(Event::VotingEnded { winner });
				return Ok(());
			}

			// Check if the user has token reserved
			let reserved_tokens = T::Currency::reserved_balance(&who);
			ensure!(reserved_tokens > 0u32.into(), Error::<T>::NotEnoughReservedTokens);

			match vote {
				Vote::Aye => {
					ensure!(
						!Self::voter_has_voted(proposal_index, &who),
						Error::<T>::VoterAlreadyVoted
					);
					// Quadratic voting logic
					let aye_votes = Self::get_aye_votes_balance(proposal_index, &who);
					let new_aye_votes = aye_votes + reserved_tokens.integer_sqrt();
					AyeVotes::<T>::set(proposal_index, &who, new_aye_votes);

					// Finally, update the total of tokens available for the voter
					let voter_balance = Self::get_voter_balance(&who);
					T::Currency::make_free_balance_be(&who, voter_balance);

					// Don't expose the voter to the public (to guarantee privacy)
					Self::deposit_event(Event::ProposalVoted { proposal_index, vote });
				},
				_ => {},
			};

			Ok(())
		}

		/*
			* Unreserve tokens
			 * @param amount: Amount of tokens to unreserve
			* @return DispatchResult

			* This function will unreserve tokens.
			* The requirements are:
				- The user must be a registered voter
				- The amount of tokens to unreserve must be greater than 0
				- The user must have enough reserved tokens to unreserve the tokens
			* To unreserve tokens, a registered voter must call this function passing the amount of tokens to unreserve
		*/
		#[pallet::call_index(5)]
		#[pallet::weight(0)]
		pub fn unreserve_tokens(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_voter_registered(&who), Error::<T>::NotRegisteredVoter);
			ensure!(amount > 0u32.into(), Error::<T>::InvalidTokensAmountToUnreserve);
			let reserved_tokens = T::Currency::reserved_balance(&who);
			ensure!(reserved_tokens >= amount, Error::<T>::NotEnoughReservedTokens);

			// Update the reserved tokens
			T::Currency::unreserve(&who, amount);
			// The "punishment" for unreserve tokens is that the voter balance will be reduced by the half of the unreserved tokens
			ensure!(
				Self::checked_div_between_balances(amount, 2u32.into()).is_ok(),
				Error::<T>::SlashFailed
			);
			T::Currency::slash(
				&who,
				Self::checked_div_between_balances(amount, 2u32.into())
					.expect("Slash already checked; QEP"),
			);
			// Update the voter balance
			let updated_balance = Self::get_voter_balance(&who);

			Self::deposit_event(Event::TokensUnreserved { who, amount, updated_balance });

			Ok(())
		}

		/*
			* Vote multiple proposals
			 * @param proposals: Vector of proposals to vote
			* @return DispatchResult

			* This function will vote multiple proposals.
			* The requirements are:
				- The user must be a registered voter
				- The proposals must be registered and active
				- The voting period must be still live
			* To vote multiple proposals, a registered voter must call this function passing the proposals to vote, the amount of tokens to vote and the vote
		*/
		#[pallet::call_index(6)]
		#[pallet::weight(0)]
		pub fn vote_multiple_proposals(
			origin: OriginFor<T>,
			proposals: Vec<(ProposalIndex, BalanceOf<T>, Vote)>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_voter_registered(&who), Error::<T>::NotRegisteredVoter);

			// Check if the proposals are registered and active
			let are_proposals_registered_and_active = proposals.iter().all(|proposal| {
				let proposal_index = proposal.0;
				Self::is_proposal_registered(proposal_index)
					&& Self::is_proposal_active(proposal_index)
			});
			ensure!(
				are_proposals_registered_and_active,
				Error::<T>::AtLeastOneProposalNotRegisteredOrNotActive
			);

			// Check if the user has token reserved
			let reserved_tokens = T::Currency::reserved_balance(&who);
			let total_tokens_to_use = proposals.iter().fold(0u32.into(), |acc, proposal| {
				acc + proposal.1
			});
			ensure!(reserved_tokens >= total_tokens_to_use, Error::<T>::NotEnoughReservedTokens);

			// Check if the user has already vote for any of the proposals
			let has_voted_for_any_proposal = proposals.iter().any(|proposal| {
				let proposal_index = proposal.0;
				Self::voter_has_voted(proposal_index, &who)
			});
			ensure!(!has_voted_for_any_proposal, Error::<T>::VoterAlreadyVoted);

			let current_block = <frame_system::Pallet<T>>::block_number();
			let proposals_are_still_active = proposals.iter().all(|proposal| {
				let proposal_index = proposal.0;
				let proposal_end_block = Self::get_proposal_end_block(proposal_index);
				current_block < proposal_end_block
			});
			// If the proposals are not active anymore, we need to update the status of the proposals to completed
			if !proposals_are_still_active {
				let winner = Self::get_winner();
				Self::update_proposal_status_to_completed(winner);
				Self::deposit_event(Event::VotingEnded { winner });

				return Ok(());
			}

			for proposal in proposals.clone() {
				let (proposal_index, tokens_to_use, vote) = proposal;

				match vote {
					Vote::Aye => {
						let aye_votes = Self::get_aye_votes_balance(proposal_index, &who);
						// Quadratic voting logic
						ensure!(
							Self::checked_add_between_balances(
								aye_votes,
								tokens_to_use.integer_sqrt()
							)
							.is_ok(),
							Error::<T>::AdditionOverflow
						);
						let new_aye_votes = Self::checked_add_between_balances(
							aye_votes,
							tokens_to_use.integer_sqrt(),
						)
						.expect("Addition already checked; QEP");

						AyeVotes::<T>::set(proposal_index, &who, new_aye_votes);

						// Finally, update the total of tokens available for the voter
						let voter_balance = Self::get_voter_balance(&who);
						T::Currency::make_free_balance_be(&who, voter_balance);

						// Don't expose the voter to the public (to guarantee privacy)
						Self::deposit_event(Event::ProposalsVoted {
							proposals: proposals.iter().map(|proposal| proposal.0).collect(),
						});
					},
					_ => {},
				};
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn is_voter_registered(who: &T::AccountId) -> bool {
			RegisteredVoters::<T>::contains_key(who)
		}
		pub fn is_proposal_registered(proposal_index: ProposalIndex) -> bool {
			Proposals::<T>::contains_key(proposal_index)
		}
		pub fn is_proposal_active(proposal_index: ProposalIndex) -> bool {
			match Proposals::<T>::get(proposal_index) {
				Some(proposal) => {
					proposal.status == ProposalStatus::InProgress
						&& Self::is_proposal_registered(proposal_index)
				},
				_ => false,
			}
		}
		pub fn get_proposal(proposal_index: ProposalIndex) -> Option<Proposal<T>> {
			Proposals::<T>::get(proposal_index)
		}
		pub fn get_proposal_status(proposal_index: ProposalIndex) -> ProposalStatus {
			Proposals::<T>::get(proposal_index)
				.map(|proposal| proposal.status)
				.expect("Proposal already checked to be registered")
		}
		pub fn get_proposal_end_block(proposal_index: ProposalIndex) -> T::BlockNumber {
			Proposals::<T>::get(proposal_index)
				.map(|proposal| proposal.end_block)
				.expect("Proposal already checked to be registered")
		}
		pub fn voter_has_voted(proposal_index: ProposalIndex, who: &T::AccountId) -> bool {
			AyeVotes::<T>::contains_key(proposal_index, who)
		}
		pub fn get_aye_votes_balance(
			proposal_index: ProposalIndex,
			who: &T::AccountId,
		) -> BalanceOf<T> {
			AyeVotes::<T>::get(proposal_index, who)
		}
		pub fn get_voter_balance(who: &T::AccountId) -> BalanceOf<T> {
			T::Currency::total_balance(who) - T::Currency::reserved_balance(who)
		}
		pub fn update_proposal_status_to_completed(proposal_index: ProposalIndex) {
			let proposal: Proposal<T> = Self::get_proposal(proposal_index)
				.expect("Proposal already checked to be registered");

			let proposal_updated: Proposal<T> = Proposal {
				proposal_index,
				text: proposal.text,
				proposer: proposal.proposer,
				end_block: proposal.end_block,
				status: ProposalStatus::Completed,
			};
			Proposals::<T>::mutate_exists(&proposal_index, |p| {
				*p = if let Some(_) = p { Some(proposal_updated) } else { None }
			});
		}
		pub fn checked_sub_between_balances(
			first_balance: BalanceOf<T>,
			second_balance: BalanceOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			first_balance
				.checked_sub(&second_balance)
				.ok_or(Error::<T>::SubstractionOverflow.into())
		}
		pub fn checked_add_between_balances(
			first_balance: BalanceOf<T>,
			second_balance: BalanceOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			first_balance
				.checked_add(&second_balance)
				.ok_or(Error::<T>::AdditionOverflow.into())
		}
		pub fn checked_div_between_balances(
			first_balance: BalanceOf<T>,
			second_balance: BalanceOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			first_balance.checked_div(&second_balance).ok_or(Error::<T>::SlashFailed.into())
		}
		// Logic to get the winner
		pub fn get_winner() -> ProposalIndex {
			let proposal_indexes = Proposals::<T>::iter().map(|(proposal_index, _)| proposal_index);
			let mut max_votes = 0u128;
			let mut winner = 0u32;
			for proposal_index in proposal_indexes {
				let total_votes: u128 = AyeVotes::<T>::iter_prefix(proposal_index)
					.map(|(_, aye_votes)| Self::balance_to_u128(aye_votes))
					.sum();
				if total_votes > max_votes {
					max_votes = total_votes;
					winner = proposal_index;
				}
			}
			winner
		}
		pub fn balance_to_u128(balance: BalanceOf<T>) -> u128 {
			balance.saturated_into::<u128>()
		}
	}
}
