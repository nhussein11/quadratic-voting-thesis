use crate::{mock::*, AccountIdOf, BalanceOf, Error, Event, ProposalIndex, ProposalStatus, Vote};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::{blake2_256, H256};

#[test]
fn register_voter() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		let fee = 5;
		let voter_id = 1;
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::is_voter_registered(&voter_id), false);
		// Dispatch a signed extrinsic.
		assert_ok!(setup_new_voter(voter_id, fee));
		// Check initial balance
		let balance = 100 - fee;
		assert_eq!(Voting::get_voter_balance(&voter_id), balance);
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::is_voter_registered(&voter_id), true);
		// Check event
		System::assert_last_event((Event::VoterRegistered { voter_id, initial_balance: balance }).into());
	});
}

#[test]
fn try_register_new_voter_with_empty_fee() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::is_voter_registered(&1), false);
		// Dispatch a signed extrinsic with empty fee.
		assert_noop!(setup_new_voter(1, 0), Error::<Test>::InsufficientFee);
		// User not registered
		assert_eq!(Voting::is_voter_registered(&1), false);
	});
}

#[test]
fn try_register_voter_already_registered() {
	new_test_ext().execute_with(|| {
		assert_eq!(Voting::is_voter_registered(&1), false);
		// Dispatch a signed extrinsic.
		assert_ok!(setup_new_voter(1, 5));
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::is_voter_registered(&1), true);
		// Dispatch a signed extrinsic.
		assert_noop!(setup_new_voter(1, 5), Error::<Test>::VoterAlreadyRegistered);
	});
}

#[test]
fn try_register_voter_with_overflow_fee(){
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::is_voter_registered(&1), false);
		// Dispatch a signed extrinsic with empty fee.
		assert_noop!(setup_new_voter(1, 101), Error::<Test>::SubstractionOverflow);
		// User not registered
		assert_eq!(Voting::is_voter_registered(&1), false);
	});
}



#[test]
fn reserve_tokens() {
	new_test_ext().execute_with(|| {
		let fee = 5;
		let voter_id = 1;
		assert_ok!(setup_new_voter(voter_id, fee));
		// Read pallet storage and assert an expected result.
		let balance = 100 - fee;
		assert_eq!(Voting::get_voter_balance(&voter_id), balance);
		// Dispatch a signed extrinsic.
		let reserved_tokens = 50;
		assert_ok!(reserve_tokens_helper(voter_id, reserved_tokens));
		// Read pallet storage and assert an expected result.
		let final_balance = balance - reserved_tokens;
		assert_eq!(Voting::get_voter_balance(&voter_id), final_balance);
		// Check event
		System::assert_last_event((Event::TokensReserved { who: 1, amount: 50 }).into());
	});
}

#[test]
fn try_to_reserve_tokens_with_not_registered_voter(){
	new_test_ext().execute_with(|| {
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::is_voter_registered(&1), false);
		// Dispatch a signed extrinsic.
		let reserved_tokens = 50;
		assert_noop!(reserve_tokens_helper(1, reserved_tokens), Error::<Test>::NotRegisteredVoter);
	});
}

#[test]
fn try_to_reserve_empty_amount_of_tokens() {
	new_test_ext().execute_with(|| {
		let fee = 5;
		assert_ok!(setup_new_voter(1, fee));
		// Read pallet storage and assert an expected result.
		let balance = 100 - fee;
		assert_eq!(Voting::get_voter_balance(&1), balance);
		// Dispatch a signed extrinsic.
		let reserved_tokens = 0;
		assert_noop!(
			reserve_tokens_helper(1, reserved_tokens),
			Error::<Test>::InvalidTokensAmountToReserve
		);
	});
}

#[test]
fn try_to_reserve_more_tokens_than_balanced() {
	new_test_ext().execute_with(|| {
		let fee = 5;
		assert_ok!(setup_new_voter(1, fee));
		// Read pallet storage and assert an expected result.
		let balance = 100 - fee;
		assert_eq!(Voting::get_voter_balance(&1), balance);
		// Reserve more tokens than the balance
		let reserved_tokens = balance + 1;
		assert_noop!(reserve_tokens_helper(1, reserved_tokens), Error::<Test>::NotEnoughBalance);
	});
}



#[test]
fn unreserve_tokens() {
	new_test_ext().execute_with(|| {
		let voter: AccountIdOf<Test> = 1;
		assert_ok!(setup_new_voter(voter, 5));
		let reserved_tokens = 50;
		let balance_before_reserve = Voting::get_voter_balance(&voter);
		assert_ok!(reserve_tokens_helper(voter, reserved_tokens));
		assert_ok!(unreserve_tokens_helper(voter, reserved_tokens));
		// Punishment for unreserving tokens is 50% of the reserved tokens (slash)
		let balance_after_reserve = balance_before_reserve - reserved_tokens / 2;
		assert_eq!(Voting::get_voter_balance(&1), balance_after_reserve);
		// Check event
		System::assert_last_event(
			(Event::TokensUnreserved { who: 1, amount: 50, updated_balance: 70 }).into(),
		);
	});
}

#[test]
fn try_to_unreserve_tokens_being_a_not_registered_voter() {
	new_test_ext().execute_with(|| {
		let voter: AccountIdOf<Test> = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Dispatch a signed extrinsic.
		let reserved_tokens = 50;
		assert_ok!(Voting::reserve_tokens(RuntimeOrigin::signed(1), reserved_tokens));
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::get_voter_balance(&1), 45);
		// Dispatch a signed extrinsic.
		let unreserved_tokens = reserved_tokens + 1;
		assert_noop!(
			unreserve_tokens_helper(2, unreserved_tokens),
			Error::<Test>::NotRegisteredVoter
		);
	});
}

#[test]
fn try_to_unreserve_empty_amount_of_tokens(){
	new_test_ext().execute_with(|| {
		let voter: AccountIdOf<Test> = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Dispatch a signed extrinsic.
		let reserved_tokens = 50;
		assert_ok!(Voting::reserve_tokens(RuntimeOrigin::signed(1), reserved_tokens));
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::get_voter_balance(&1), 45);
		// Dispatch a signed extrinsic.
		let unreserved_tokens = 0;
		assert_noop!(
			unreserve_tokens_helper(voter, unreserved_tokens),
			Error::<Test>::InvalidTokensAmountToUnreserve
		);
	});
}

#[test]
fn try_to_unreserve_more_tokens_than_reserved() {
	new_test_ext().execute_with(|| {
		let voter: AccountIdOf<Test> = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Dispatch a signed extrinsic.
		let reserved_tokens = 50;
		assert_ok!(Voting::reserve_tokens(RuntimeOrigin::signed(1), reserved_tokens));
		// Read pallet storage and assert an expected result.
		assert_eq!(Voting::get_voter_balance(&1), 45);
		// Dispatch a signed extrinsic.
		let unreserved_tokens = reserved_tokens + 1;
		assert_noop!(
			unreserve_tokens_helper(voter, unreserved_tokens),
			Error::<Test>::NotEnoughReservedTokens
		);
	});
}



#[test]
fn create_new_proposal() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Check proposal status (the proposal index is 1 because it's the first proposal created)
		assert_eq!(Voting::get_proposal_status(1), ProposalStatus::NotStarted);
		// Check event
		System::assert_last_event(
			(Event::NewProposalCreated {
				proposal_index: 1,
				text: "Let's use blockchain to create a better world!"
					.using_encoded(blake2_256)
					.into(),
				end_block: Voting::get_proposal_end_block(1),
			})
			.into(),
		);
	})
}

#[test]
fn try_to_create_new_proposal_with_not_registered_voter() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		// Voter 1 has not been registered
		// Create proposal with non registered voter
		assert_noop!(
			create_proposal(voter, "Let's use blockchain to create a better world!"),
			Error::<Test>::NotRegisteredVoter
		);
	})
}




#[test]
fn start_proposal() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		let fee_to_new_voter = 5;
		assert_ok!(setup_new_voter(voter, fee_to_new_voter));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		let fee_to_start_proposal = 10;
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, fee_to_start_proposal));
		// Check proposal status
		assert_eq!(Voting::get_proposal_status(1), ProposalStatus::InProgress);
		// Check voter balance
		let final_balance = 100 - fee_to_new_voter - fee_to_start_proposal;
		assert_eq!(Voting::get_voter_balance(&voter), final_balance);
		// Check event
		System::assert_last_event((Event::ProposalStarted { proposal_index: 1 }).into());
	})
}

#[test]
fn try_to_start_proposal_with_not_registered_voter() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		// Voter 1 has not been registered
		// Start proposal with non registered voter
		assert_noop!(start_proposal_helper(voter, 1, 10), Error::<Test>::NotRegisteredVoter);
	})
}

#[test]
fn try_to_start_not_existing_proposal() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		let fee_to_new_voter = 5;
		assert_ok!(setup_new_voter(voter, fee_to_new_voter));
		// Start proposal without creating it
		assert_noop!(start_proposal_helper(voter, 1, 10), Error::<Test>::ProposalNotFound);
	})
}

#[test]
fn try_to_start_proposal_with_not_enough_balance() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		let fee_to_new_voter = 5;
		assert_ok!(setup_new_voter(voter, fee_to_new_voter));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		let fee_to_start_proposal = 100;
		// Start proposal
		assert_noop!(
			start_proposal_helper(voter, 1, fee_to_start_proposal),
			Error::<Test>::NotEnoughBalance
		);
	})
}

#[test]
fn try_to_start_proposal_that_it_is_already_started() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		let fee_to_new_voter = 5;
		assert_ok!(setup_new_voter(voter, fee_to_new_voter));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		let fee_to_start_proposal = 10;
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, fee_to_start_proposal));
		// Try to start proposal again
		assert_noop!(
			start_proposal_helper(voter, 1, fee_to_start_proposal),
			Error::<Test>::ProposalAlreadyStarted
		);
	})
}

#[test]
fn try_to_start_proposal_with_empty_fee() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		let fee_to_new_voter = 5;
		assert_ok!(setup_new_voter(voter, fee_to_new_voter));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		let fee_to_start_proposal = 0;
		// Start proposal
		assert_noop!(
			start_proposal_helper(voter, 1, fee_to_start_proposal),
			Error::<Test>::InsufficientFee
		);
	})
}




#[test]
fn vote_proposal_with_tokens_reserved() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Reserve Tokens
		assert_ok!(reserve_tokens_helper(voter, 50));
		// Vote proposal
		assert_ok!(vote_proposal(voter, 1, Vote::Aye));
		// Check event
		System::assert_last_event(
			(Event::ProposalVoted { proposal_index: 1, vote: Vote::Aye }).into(),
		);
	})
}

#[test]
fn try_to_vote_proposal_with_not_registered_voter() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		// Voter 1 has not been registered
		assert_ok!(setup_new_voter(voter, 5));
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		assert_ok!(start_proposal_helper(voter, 1, 10));

		let new_voter_not_registered = 2;
		// Vote proposal with non registered voter
		assert_noop!(
			vote_proposal(new_voter_not_registered, 1, Vote::Aye),
			Error::<Test>::NotRegisteredVoter
		);
	})
}

#[test]
fn try_to_vote_proposal_not_registered() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Vote proposal
		assert_noop!(vote_proposal(voter, 2, Vote::Aye), Error::<Test>::ProposalNotFound);
	})
}

#[test]
fn try_to_vote_proposal_not_active() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Create other proposal but not start it
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Vote proposal
		assert_noop!(vote_proposal(voter, 2, Vote::Aye), Error::<Test>::ProposalNotActive);
	})
}

#[test]
fn try_vote_proposal_without_tokens_reserved() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(1), 1, 10));
		// Vote proposal
		assert_noop!(
			Voting::vote_proposal(RuntimeOrigin::signed(1), 1, Vote::Aye),
			Error::<Test>::NotEnoughReservedTokens
		);
	})
}

#[test]
fn try_to_vote_proposal_twice() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Reserve Tokens
		assert_ok!(reserve_tokens_helper(voter, 50));
		// Vote proposal
		assert_ok!(vote_proposal(voter, 1, Vote::Aye));
		// Vote proposal again
		assert_noop!(vote_proposal(voter, 1, Vote::Aye), Error::<Test>::VoterAlreadyVoted);
	})
}

#[test]
fn vote_repetitive_proposals_without_tokens_reserved() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Vote proposal
		assert_noop!(vote_proposal(voter, 1, Vote::Aye), Error::<Test>::NotEnoughReservedTokens);
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter, 2, 10));
		// Vote other proposal
		assert_noop!(vote_proposal(voter, 2, Vote::Aye), Error::<Test>::NotEnoughReservedTokens);
	})
}

// I wrote a longer test to show the interaction with more voters and also get the proposal winner
#[test]
fn check_proposal_winner() {
	new_test_ext().execute_with(|| {
		let voter_1 = 1;
		let voter_2 = 2;
		let voter_3 = 3;
		let voter_4 = 4;

		assert_ok!(setup_new_voter(voter_1, 5));
		assert_ok!(setup_new_voter(voter_2, 5));
		assert_ok!(setup_new_voter(voter_3, 5));
		assert_ok!(setup_new_voter(voter_4, 5));

		// Let's create a proposal by voter 1
		assert_ok!(create_proposal(voter_1, "Let's use blockchain to create a better world!"));
		// And then create another proposal by voter 2
		assert_ok!(create_proposal(voter_2, "Blockchain is the future!"));
		// Start that proposal 1
		assert_ok!(start_proposal_helper(voter_1, 1, 10));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter_2, 2, 10));

		// Proposal 1:
		assert_ok!(reserve_tokens_helper(voter_1, 50));
		assert_ok!(vote_proposal(voter_1, 1, Vote::Aye));

		assert_ok!(reserve_tokens_helper(voter_3, 40));
		assert_ok!(vote_proposal(voter_3, 1, Vote::Aye));

		// Proposal 2:
		assert_ok!(reserve_tokens_helper(voter_2, 40));
		assert_ok!(vote_proposal(voter_2, 2, Vote::Aye));
		assert_ok!(reserve_tokens_helper(voter_4, 30));
		assert_ok!(vote_proposal(voter_4, 2, Vote::Aye));
		assert_ok!(reserve_tokens_helper(voter_3, 50));
		assert_ok!(vote_proposal(voter_3, 2, Vote::Aye));

		// Go past voting period
		System::set_block_number(200);
		assert_ok!(reserve_tokens_helper(voter_2, 10));
		assert_ok!(vote_proposal(voter_2, 2, Vote::Aye));
		// End voting
		assert_eq!(Voting::get_winner(), 2);
		// Check event
		System::assert_last_event((Event::VotingEnded { winner: 2 }).into());
	})
}




#[test]
fn vote_multiples_proposals(){
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter, 2, 10));
		// Reserve Tokens
		assert_ok!(reserve_tokens_helper(voter, 70));
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 50, Vote::Aye), (2, 20, Vote::Aye)];
		assert_ok!(vote_multiple_proposals_helper(voter, proposals));
		// Check event
		System::assert_last_event((Event::ProposalsVoted { proposals: [1,2].into() }).into());
	})
}

#[test]
fn vote_multiples_proposal_when_voting_has_ended(){
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter, 2, 10));
		// Reserve Tokens
		assert_ok!(reserve_tokens_helper(voter, 60));
		// System::assert_last_event((Event::TokensReserved { who: voter, amount: 60 }).into());
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 10, Vote::Aye), (2, 5, Vote::Aye)];
		assert_ok!(vote_multiple_proposals_helper(voter, proposals));
		// System::assert_last_event((Event::ProposalsVoted { proposals: [1,2].into() }).into());
		// Check event
		// System::assert_last_event((Event::winners(1, 2)).into());
		// Go past voting period
		let voter  = 2;
		assert_ok!(setup_new_voter(voter, 5));
		// Reserve Tokens
		assert_ok!(reserve_tokens_helper(voter, 10));
		System::set_block_number(250);
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
		vec![(1, 5, Vote::Aye), (2, 4, Vote::Aye)];

		assert_ok!(vote_multiple_proposals_helper(voter, proposals));
		// Check events
		System::assert_last_event((Event::VotingEnded { winner: 1 }).into());
	})
}

#[test]
fn try_to_vote_multiple_proposals_with_not_registered_voter() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter, 2, 10));
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 10, Vote::Aye), (2, 15, Vote::Aye)];
		assert_noop!(
			// 2 is not a registered voter
			vote_multiple_proposals_helper(2, proposals),
			Error::<Test>::NotRegisteredVoter
		);
	})
}

#[test]
fn try_to_vote_multiple_proposals_with_proposal_not_found() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Vote proposals that have never been created
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 10, Vote::Aye), (2, 15, Vote::Aye)];
		assert_noop!(
			vote_multiple_proposals_helper(voter, proposals.clone()),
			Error::<Test>::AtLeastOneProposalNotRegisteredOrNotActive
		);
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Proposal 1 is created and started but 2 doesn't exist
		assert_noop!(
			vote_multiple_proposals_helper(voter, proposals),
			Error::<Test>::AtLeastOneProposalNotRegisteredOrNotActive
		);
	})
}

#[test]
fn try_to_vote_multiple_proposals_with_proposal_not_started() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 10, Vote::Aye), (2, 15, Vote::Aye)];
		assert_noop!(
			vote_multiple_proposals_helper(voter, proposals.clone()),
			Error::<Test>::AtLeastOneProposalNotRegisteredOrNotActive
		);
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// But not the other one
		assert_noop!(
			vote_multiple_proposals_helper(voter, proposals.clone()),
			Error::<Test>::AtLeastOneProposalNotRegisteredOrNotActive
		);
	})
}

#[test]
fn try_to_vote_multiples_proposals_without_enough_token_reserved() {
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter, 2, 10));
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 10, Vote::Aye), (2, 15, Vote::Aye)];
		assert_noop!(
			vote_multiple_proposals_helper(voter, proposals),
			Error::<Test>::NotEnoughReservedTokens
		);
	})
}

#[test]
fn try_to_vote_multiple_proposals_when_the_voter_already_had_voted_one(){
	new_test_ext().execute_with(|| {
		let voter = 1;
		assert_ok!(setup_new_voter(voter, 5));
		// Create proposal
		assert_ok!(create_proposal(voter, "Let's use blockchain to create a better world!"));
		// Start proposal
		assert_ok!(start_proposal_helper(voter, 1, 10));
		// Create other proposal
		assert_ok!(create_proposal(voter, "Blockchain is the future!"));
		// Start other proposal
		assert_ok!(start_proposal_helper(voter, 2, 10));
		// Reserve some tokens
		assert_ok!(reserve_tokens_helper(voter, 75));
		// Vote proposal 1
		assert_ok!(vote_proposal(voter, 1, Vote::Aye));
		// Vote proposals at once
		let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> =
			vec![(1, 10, Vote::Aye), (2, 15, Vote::Aye)];
		assert_noop!(
			vote_multiple_proposals_helper(voter, proposals),
			Error::<Test>::VoterAlreadyVoted
		);
	})
}




// Helper Functions
fn setup_new_voter(
	voter: AccountIdOf<Test>,
	fee: BalanceOf<Test>,
) -> Result<(), sp_runtime::DispatchError> {
	System::set_block_number(1);
	Voting::register_voter(RawOrigin::Root.into(),voter, fee)
}
fn reserve_tokens_helper(
	voter: AccountIdOf<Test>,
	amount: BalanceOf<Test>,
) -> Result<(), sp_runtime::DispatchError> {
	Voting::reserve_tokens(RuntimeOrigin::signed(voter), amount)
}
fn unreserve_tokens_helper(
	voter: AccountIdOf<Test>,
	amount: BalanceOf<Test>,
) -> Result<(), sp_runtime::DispatchError> {
	Voting::unreserve_tokens(RuntimeOrigin::signed(voter), amount)
}
fn create_proposal(voter: AccountIdOf<Test>, text: &str) -> Result<(), sp_runtime::DispatchError> {
	let hashed_text: H256 = text.using_encoded(blake2_256).into();
	Voting::create_proposal(RuntimeOrigin::signed(voter), hashed_text)
}
fn start_proposal_helper(
	voter: AccountIdOf<Test>,
	proposal_index: ProposalIndex,
	fee: BalanceOf<Test>,
) -> Result<(), sp_runtime::DispatchError> {
	Voting::start_proposal(RuntimeOrigin::signed(voter), proposal_index, fee)
}
fn vote_proposal(
	voter: AccountIdOf<Test>,
	proposal_index: ProposalIndex,
	vote: Vote,
) -> Result<(), sp_runtime::DispatchError> {
	Voting::vote_proposal(RuntimeOrigin::signed(voter), proposal_index, vote)
}
fn vote_multiple_proposals_helper(
	voter: AccountIdOf<Test>,
	proposals: Vec<(ProposalIndex, BalanceOf<Test>,Vote)>,
) -> Result<(), sp_runtime::DispatchError> {
	Voting::vote_multiple_proposals(RuntimeOrigin::signed(voter), proposals)
}
