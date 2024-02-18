
# Quadratic Voting Pallet for Substrate
- [Quadratic Voting](#quadratic-voting)
    + [How does it work?](#how-does-it-work-)
    + [Register a new voter](#register-a-new-voter)
    + [Create a proposal](#create-a-proposal)
    + [Start a proposal](#start-a-proposal)
    + [Reserve tokens](#reserve-tokens)
    + [Vote a proposal](#vote-a-proposal)
    + [Unreserve tokens](#unreserve-tokens)
    + [Extra](#extra)
- [Ideas for future improvements](#ideas-for-future-improvements)
- [How to run the project](#how-to-run-the-project)
- [How to test the project](#how-to-test-the-project)
- [Resources that I found useful](#resources-that-i-found-useful)




# Quadratic Voting

Allows people to express the relative strength of their preferences for different issues in a democratic process.
People are more likely to vote strongly not only about issues they care more about, but issues they know more about

### How does it work?

I take the initial idea of the Quadratic Voting System and then I implemented it in Substrate with some slightly differences to do it more interesting (at least for me).
So, this version of the Quadratic Voting is based on the following assumptions: 

Suppose you have different kind of problems:
    - Spend more money on education
    - Spend more money on health
    - Spend more money on defense
    - Generate more jobs
    - Try to reduce the unemployment rate
    - Invest in renewable energy
    - Invest in new technologies

And if you want to vote, you can do it depending on the most important problems for you.

So, my pallet works in the following way:

### Register a new voter

Firstly, I considered that only the "root" user can register new voters. To do that, the "root" needs to call the extrinsic "register_voter" and provide the voter id (AccountId of the new voter to register) with a fee (Balance). Then, after validate that the root user has provided a valid fee (greater than zero), a new user voter is created with the a balance of tokens equals to some "initial balance" (for my project I choose 100 tokens) minus the fee.

```bash
    balance = initial_balance - fee 
```

Currently, I'm using a StorageMap item to keep a register of the voters in the system. Every key there is the AccountId of the voter and the value is simply a boolean.

* ***Note***: I didn't specify anything about the amount of fee necessary to register a new voter (I considered it as something merely symbolic, just to express some restrictions or limitations to the root user. However, I will talk more about this in the "Ideas for future improvements" section).

### Create a proposal

Then, I also assumed that every voter has the possibility to create a proposal. It means that every voter can create a proposal to expose a problem to the community. To do that, the voter needs to call the extrinsic "create_proposal" and provide the hash of the text that describes the proposal. Then, the extrinsic will check if the voter is already a registered voter. I believe that creating a proposal shouldn't require payment. So, after validate that the voter is already a registered voter, a new proposal is created with the hash of the text that describes the proposal. 
In my design of the pallet, I considered that each proposal has a field called "end_block", which refers to
the block number when the proposal will be closed. So, the proposal will be closed when the current block number is greater than the "end_block" field.

* ***Note***: At the moment, the voting period is fixed by the following:

```bash
    end_block = current_block + voting_period 
```

*Where "voting_period" refers to the SLOT_DURATION (in my project, I choose 1 * HOURS, having MILLISECS_PER_BLOCK = 6000 ).*

### Start a proposal

To start a proposal, a voter needs to call the extrinsic "start_proposal" and provide the hash of the text that describes the proposal. Then, the extrinsic will check if the voter is already a registered voter and if the proposal exists. If the proposal exists, the extrinsic will check if the proposal is already started. And if the proposal is already started, the extrinsic will return an error. Otherwise, the extrinsic will start the proposal. 
To start a proposal, the voter needs to pay a fee (again, I considered the fee as something merely symbolic, just to express some restrictions to the voters. Honestly, I think that it's a good idea for some potential cases where a (probably malicious) voter wants to start a proposal to spam the system).
So, assuming that the voter has enough tokens to pay the fee, the new balance of the voter will be:

```bash 
    balance = balance - fee 
```


### Reserve tokens

If a voter wants to submit a vote, previously he/she should had reserved some tokens to vote for a proposal. To do that, the voter needs to call the extrinsic "reserve_tokens" and provide the number of tokens that he/she wants to reserve. Then, the extrinsic will check if the voter is already a registered voter and if he/she has enough tokens to reserve the number of tokens that he/she wants to reserve. If the voter has enough tokens, the extrinsic will reserve the tokens. Otherwise, the extrinsic will return an error. So now, the balance of the voter will be:

```bash 
    balance = balance - tokens_to_reserve 
``` 

### Vote a proposal

So, the idea is that you can vote for a proposal with a number of reserved tokens (e.g. 100 tokens) and then, the number of tokens that you reserved for a proposal will be squared (e.g. sqrt(100) = 10 votes) to count it. That's the main idea of the Quadratic Voting System.

Assuming that everything is ok, the tokens that you reserved will be squared to count it as a vote. So, the number of votes for a proposal will be:

```bash
    votes = sqrt(reserved_tokens)
```

And then, the balance will be updated as follows:

```bash 
    balance = balance - reserved_tokens 
```

* ***Note***: In my project, I'm only considering the possibility to vote "Aye" for a proposal (even though I create the "Vote" enum with Nay and Abstain options too). If the voter wants to vote "Nay" or "Abstain", it just will be ignored (I'll talk about this in the "Ideas for future improvements" section).    

* ***Note***: If the voter want to vote for a proposal that is already closed, the extrinsic will calculate the number of ayes votes for each proposal and then, it will return the proposal with the highest number of ayes votes. That's how I'm considering the ***winner proposal***.

### Unreserve tokens

If a voter wants to unreserve some tokens, he/she can do it. However, in my design of the pallet, I considered that the voter will lose 50% of the tokens that he/she unreserve. So, the balance will be updated as follows:

``` 
    balance = balance + (tokens_to_unreserve / 2) 
```

In my opinion, I believe that this "punishment" is necessary to incentivize the voters to reserve the tokens that they will use to vote for a proposal. Otherwise, the voters could reserve a lot of tokens and then, unreserve them to use them in another proposal, making the system inefficient. *The idea is just to make the voters think twice before unreserve tokens*. 

### Extra: Vote for multiple proposals

* In this PR: https://github.com/Polkadot-Blockchain-Academy/pba-assignment-3-nhussein11/pull/2 , I added a new feature to the Quadratic Voting System. 
    Now, the voters can vote for multiple proposals. To do that, the voters need to call the extrinsic "vote_multiple_proposals" and provide a vector of tuples, where each tuple contains a proposal index of the proposal that they want to vote for and the number of tokens that the voter wants to reserve for that proposal. 
    For instance, the line below shows how to vote for two proposals, where the first proposal has index 1, the second proposal has index 2 and the voter wants to reserve 10 tokens for the first proposal and 5 tokens for the second proposal:
    ```bash
        let proposals: Vec<(ProposalIndex, BalanceOf<Test>, Vote)> = vec![(1, 10, Vote::Aye), (2, 5, Vote::Aye)];
    ```
    Then, the extrinsic will check if the voting is still alive for each proposal given, if the voter is already a registered voter and if he/she has enough tokens to reserve the number of tokens that he/she wants to reserve for each proposal. If the voter has enough tokens, the extrinsic will reserve the tokens for each proposal. Otherwise, the extrinsic will return an error. So now, the balance of the voter will be:
    
    ```bash 
        balance = balance - tokens_to_reserve 
    ```
    * ***Note***: Same as I considered for vote one single proposal, If the voter wants to vote for a proposal that is already closed, the extrinsic will calculate the number of ayes votes for each proposal and then, it will return the proposal with the highest number of ayes votes. That's how I'm considering the ***winner proposal***.



&rarr; ***General Note***: All the extrinsics that I mentioned above are commented in the code, following the Substrate documentation style. So, you can check the code to see more details about the implementation of each extrinsic. 

# Ideas for future improvements
If I had more time, I would like to implement the following features:

* Add a new feature to allows the voters to vote "Nay" or "Abstain" for a proposal. Considering that some "proposals" could be positive for some people, but negative for others. In my opinion, these could be a good way to "democratize" even more the voting system.

* If we have a lot of proposals, and some of them are more important than others, I think that it could be a good idea to replace the voting duration (a certain amount of blocks) to a voting consensus (e.g. 3/4 of the voters have voted for a proposal). In this way, we can avoid that some proposals are "blocked" for a long time, just because the voting duration is too long.

* And finally, one additional feature that could be implemented is the ability for voters to delegate their voting power to another voter. This would allow individuals who may not have the time or resources to research and make informed decisions about proposals to delegate their voting power to someone they trust who has more knowledge or expertise on the subject. This would also increase voter turnout and participation in the voting process.

# How to run the project

- Install Rust
    ```
        curl https://sh.rustup.rs -sSf | sh
    ```

- Install Substrate
    ```
        curl https://getsubstrate.io -sSf | bash -s -- --fast
    ```

- Clone the project
    ```
        git clone https://github.com/Polkadot-Blockchain-Academy/pba-assignment-3-nhussein11.git 
    ```
    
- Open the project
    ```
        cd substrate-node-template
    ```

- Build the project
    ```
        cargo build --release 
    ```

- Run the project
    ```
        ./target/release/substrate-node-template --dev
    ``` 

# How to test the project

I wrote a suite of unit test to test the project. The idea is to test the main functionalities of the project. For each extrinsic, I wrote a test to check if the extrinsic is working as expected (happy path). And then, I also wrote a test to check if the extrinsic is returning an error when it should (unhappy path). 
For instance, if I have the extrinsic "start_proposal", I wrote the following tests:
* start_proposal: to check if the extrinsic is working as expected (happy path). I also check the event that should be emitted and how the storage should be updated.

* And then, for each error that the extrinsic can return, the test:

    * try_to_start_proposal_with_not_registered_voter &rarr; **Error::NotRegisteredVoter**
    * try_to_start_not_existing_proposal &rarr; **Error::ProposalNotFound**
    * try_to_start_proposal_with_not_enough_balance &rarr; **Error::NotEnoughBalance**
    * try_to_start_proposal_that_it_is_already_started &rarr; **Error::ProposalAlreadyStarted**
    * try_to_start_proposal_with_empty_fee &rarr; **Error::InsufficientFee**


To run the **all** tests, you need to run the following command:

```bash
    cargo test -p pallet-voting
```

And, if you want to run a **specific** test, you can run the following command:

```bash
    cargo test -p pallet-voting -- <test_name>
```

For example, if you want to run the test "test_create_proposal", you can run the following command:
```bash
    cargo test -p pallet-voting -- check_initial_balance
```
    
# Resources that I found useful
- To understand the concept of Quadratic Voting: 
    - https://www.youtube.com/watch?v=pjbakxIvGFA
    - https://timdaub.github.io/2022/03/27/the-user-experience-problems-of-quadratic-voting/ 
    - https://blog.tally.xyz/a-simple-guide-to-quadratic-voting-327b52addde1
    - https://www.ias.edu/sites/default/files/sss/pdfs/Rodrik/workshop%2014-15/Weyl-Quadratic_Voting.pdf
    - https://www.naukri.com/learning/articles/quadratic-voting-all-that-you-need-to-know/ 

- Substrate and FRAME:
    - https://substrate.dev/docs/en/knowledgebase/runtime/frame
    - https://substrate.dev/docs/en/knowledgebase/runtime/frame#pallets
    - https://paritytech.github.io/substrate
