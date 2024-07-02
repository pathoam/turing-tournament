#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("1gofU3bezQhP9anagsc3HaRw1few2qZbUNMmF4kLPkh");

#[program]
pub mod turing {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, bump: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;
        game.bump = bump;
        game.authority = *ctx.accounts.authority.key;

        // Initialize the game's user account
        let game_user_account = &mut ctx.accounts.game_user_account;
        game_user_account.user = *ctx.accounts.game.key;
        game_user_account.balance = 0;

        Ok(())
    }

    pub fn create_user_account(ctx: Context<CreateUserAccount>) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        user_account.user = *ctx.accounts.user.key;
        user_account.balance = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.game_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let user_account = &mut ctx.accounts.user_account;
        user_account.balance = user_account.balance.checked_add(amount).unwrap();
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let user_account = &mut ctx.accounts.user_account;
        require!(user_account.balance >= amount, CustomError::InsufficientFunds);

        let cpi_accounts = Transfer {
            from: ctx.accounts.game_token_account.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.game.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let seeds = &[b"game".as_ref(), &[ctx.accounts.game.bump]];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        user_account.balance = user_account.balance.checked_sub(amount).unwrap();
        Ok(())
    }

    pub fn attest_outcome(ctx: Context<AttestOutcome>, winner: Option<Pubkey>, loser: Option<Pubkey>, stake: u64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        require!(ctx.accounts.authority.key == game.authority, CustomError::Unauthorized);

        let game_user_account = &mut ctx.accounts.game_user_account;
        let game_fee = stake / 10; // 10% fee to the game
        let net_stake = stake - game_fee;

        if let Some(winner_key) = winner {
            let winner_account = &mut ctx.accounts.user_accounts.iter_mut().find(|acc| acc.user == winner_key).unwrap();
            winner_account.balance = winner_account.balance.checked_add(net_stake).unwrap();
        }

        if let Some(loser_key) = loser {
            let loser_account = &mut ctx.accounts.user_accounts.iter_mut().find(|acc| acc.user == loser_key).unwrap();
            loser_account.balance = loser_account.balance.checked_sub(stake).unwrap();
        }

        game_user_account.balance = game_user_account.balance.checked_add(game_fee).unwrap();

        Ok(())
    }

    pub fn admin_deposit(ctx: Context<AdminDeposit>, amount: u64) -> Result<()> {
        let cpi_accounts = Transfer {
            from: ctx.accounts.admin_token_account.to_account_info(),
            to: ctx.accounts.game_token_account.to_account_info(),
            authority: ctx.accounts.admin.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let game_user_account = &mut ctx.accounts.game_user_account;
        game_user_account.balance = game_user_account.balance.checked_add(amount).unwrap();
        Ok(())
    }

    pub fn admin_withdraw(ctx: Context<AdminWithdraw>, amount: u64) -> Result<()> {
        let game_user_account = &mut ctx.accounts.game_user_account;
        require!(game_user_account.balance >= amount, CustomError::InsufficientFunds);

        let cpi_accounts = Transfer {
            from: ctx.accounts.game_token_account.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
            authority: ctx.accounts.game.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let seeds = &[b"game".as_ref(), &[ctx.accounts.game.bump]];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        game_user_account.balance = game_user_account.balance.checked_sub(amount).unwrap();
        Ok(())
    }
}

#[derive(Accounts)]
