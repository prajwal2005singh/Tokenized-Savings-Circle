# 💰 Tokenized Savings Circle (Digital ROSCA) on Soroban

## Overview

This project implements a **Tokenized Savings Circle** (also known as a Digital "Chit Fund" or Rotating Savings and Credit Association - **ROSCA**) using a Soroban smart contract on the Stellar network.

It allows a group of pre-defined members to pool a fixed amount of tokens every cycle. In each cycle, the entire pooled amount is paid out to exactly one member in a round-robin fashion. The contract automates collection, tracks reputation, enforces penalties for late or missing deposits, and ensures a transparent, on-chain mechanism for group savings.

---

## ✨ Features

* **On-chain Group Membership:** Fixed number of members stored and managed by the contract.
* **Automated Pot Collection:** Defines a fixed deposit amount and token asset.
* **Round-Robin Payout:** Ensures fair and predictable distribution of the pooled funds to members.
* **Penalty System:** Implements different penalties for late vs. missing deposits, accumulating on-chain.
* **Reputation Scoring:** Tracks member reliability based on successful and missed deposits.
* **Cycle Scheduling:** Logic to ensure the cycle advances only once per defined interval via an external `execute_cycle` call.
* **Emergency Pause:** An owner-controlled flag to temporarily halt critical contract operations.
* **Refund Mechanism:** Allows members to claim accrued refunds/penalties if the circle is paused or ended (future extension).

---

## 🛠 Smart Contract Functions

The Soroban smart contract is the core of the system, managing state, logic, and token transfers.

| Function | Description | Access Control |
| :--- | :--- | :--- |
| `create_circle` | Initializes a new savings circle with members, deposit amount, and cycle interval. | Owner/Anyone |
| `join_circle` | Allows a participant to confirm their spot *before* the join deadline. | Member |
| `deposit` | Transfers the fixed deposit amount for the current cycle to the contract's escrow. | Member |
| `execute_cycle` | Advances the circle to the next cycle, performs payout, and applies penalties if deposits were missed. | Relayer/Frontend |
| `claim_refund` | Allows a member to claim any accrued penalties or refunds. | Member |
| `pause` | Sets the emergency pause flag, halting all critical operations. | Owner |
| `unpause` | Resets the emergency pause flag. | Owner |
| `get_circle` | Reads the current state of the circle (cycle number, next payout, etc.). | Anyone |
| `get_member_state` | Reads a specific member's state, including reputation and accrued penalties. | Anyone |

---

## 📁 Folder Structure