import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LiquidityPool } from "../target/types/liquidity_pool";
import {mintTo , createMint, createAccount, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, getAccount} from "@solana/spl-token";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";

describe("liquidity-pool", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider);

  const program = anchor.workspace.liquidityPool as Program<LiquidityPool>;
  const LAMPORTS_PER_SOL = 100_000_000_0
  const wallet = provider.wallet as NodeWallet;
  const USDC_DECIMALS = 100_000_0

  let usdc_mint: anchor.web3.PublicKey;
  let wrapped_solana_mint: anchor.web3.PublicKey;

  let userAUsdcAta: anchor.web3.PublicKey;
  let userBUsdcAta: anchor.web3.PublicKey;
  let swapUserUsdcAta: anchor.web3.PublicKey;

  let userASolAta: anchor.web3.PublicKey;
  let userBSolAta: anchor.web3.PublicKey;
  let swapUserSolAta: anchor.web3.PublicKey;

  let poolUsdcAta: anchor.web3.PublicKey;
  let poolSolAta:anchor.web3.PublicKey;

  let userAPda: anchor.web3.PublicKey;
  let userBPda: anchor.web3.PublicKey;
  let pool_pda: anchor.web3.PublicKey;

  const userA = anchor.web3.Keypair.generate();
  const userB = anchor.web3.Keypair.generate();
  const swapUser = anchor.web3.Keypair.generate()

  it("airdrop some sol to the user wallet" ,async () => {
    const userAtx = await provider.connection.requestAirdrop(
      userA.publicKey, 
      100 * LAMPORTS_PER_SOL
    )
    await provider.connection.confirmTransaction(userAtx);

    const userBtx = await provider.connection.requestAirdrop(
      userB.publicKey,
      100 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(userBtx)

    const swapUserTx = await provider.connection.requestAirdrop(
      swapUser.publicKey, 
      100 * LAMPORTS_PER_SOL
    )

    await provider.connection.confirmTransaction(swapUserTx);
  })

  it("initialising the mint and create ata", async () => {
     usdc_mint = await createMint(
      provider.connection, 
      wallet.payer, 
      wallet.publicKey, 
      wallet.publicKey, 
      6
    )

     wrapped_solana_mint = await createMint(
      provider.connection, 
      wallet.payer, 
      wallet.publicKey, 
      wallet.publicKey, 
      9
    )

    userAUsdcAta = (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      wallet.payer, 
      usdc_mint ,
      userA.publicKey, 
    )).address

    userBUsdcAta = (await getOrCreateAssociatedTokenAccount(
      provider.connection, 
      wallet.payer, 
      usdc_mint, 
      userB.publicKey, 
    )).address

    swapUserUsdcAta = (await getOrCreateAssociatedTokenAccount(
      provider.connection, 
      wallet.payer, 
      usdc_mint, 
      swapUser.publicKey, 
    )).address

    userASolAta = (await getOrCreateAssociatedTokenAccount(
      provider.connection,
      wallet.payer, 
      wrapped_solana_mint ,
      userA.publicKey, 
    )).address

    userBSolAta = (await getOrCreateAssociatedTokenAccount(
      provider.connection, 
      wallet.payer, 
      wrapped_solana_mint, 
      userB.publicKey, 
    )).address

    let [pda_key, bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), usdc_mint.toBuffer(), wrapped_solana_mint.toBuffer()],
      program.programId
    )

    pool_pda = pda_key;

    poolUsdcAta = getAssociatedTokenAddressSync(
      usdc_mint, 
      pool_pda,
      true
    )
    poolSolAta = getAssociatedTokenAddressSync(
      wrapped_solana_mint, 
      pool_pda,
      true
    )
  })

  it("mint some wrapped solana token and usdc token into the user ata ", async () => {
    const mintUsdc1 = await mintTo(
      provider.connection, 
      wallet.payer, 
      usdc_mint, 
      userAUsdcAta, 
      wallet.payer, 
      10000000 * USDC_DECIMALS
    )

    const mintUsdc2 = await mintTo(
      provider.connection, 
      wallet.payer, 
      usdc_mint, 
      userBUsdcAta, 
      wallet.payer, 
      10000000 * USDC_DECIMALS
    )

    const mintSolana1 = await mintTo(
      provider.connection, 
      wallet.payer, 
      wrapped_solana_mint, 
      userASolAta, 
      wallet.payer, 
      10000000 * LAMPORTS_PER_SOL
    )

    const tx2 = await mintTo(
      provider.connection, 
      wallet.payer, 
      wrapped_solana_mint, 
      userBSolAta, 
      wallet.payer, 
      10000000 * LAMPORTS_PER_SOL
    )

    const swaptx = await mintTo(
      provider.connection,
      wallet.payer, 
      usdc_mint, 
      swapUserUsdcAta, 
      wallet.payer, 
      10000000 * USDC_DECIMALS
    )
  })
  
  it("get pda address for the liquidity provider", async () => {

    [userAPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), userA.publicKey.toBuffer()], 
      program.programId
    ) 

    let a = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), userB.publicKey.toBuffer()],
      program.programId
    )
    userBPda = a[0]

    

  })

  it("Is initialized pool!", async () => {
    // Add your test here.
    let usdc_amount = 600 * USDC_DECIMALS;
    let sol_amount = 12 * LAMPORTS_PER_SOL

    const tx = await program.methods.deposit(new anchor.BN(usdc_amount),new anchor.BN(sol_amount))
    .accountsPartial({
      signer: userA.publicKey, 
      usdcMint: usdc_mint, 
      wrappedSolMint: wrapped_solana_mint,
      userUsdcAta: userAUsdcAta, 
      userSolAta: userASolAta,
      userPda: userAPda, 
      poolPda: pool_pda, 
      poolSolAta: poolSolAta, 
      poolUsdcAta: poolUsdcAta,
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .signers([userA])
    .rpc();
    console.log("Your transaction signature", tx);

    const poolusdcAccountInfo = await getAccount(provider.connection, poolUsdcAta)
    const poolSolAtaInfo = await getAccount(provider.connection,poolSolAta)


    console.log("This is the balance  of the liquidity pool usdc after swapping", Number(poolusdcAccountInfo.amount) / USDC_DECIMALS)
    console.log("This is the balance of the liquidity pool solana after swapping", Number(poolSolAtaInfo.amount) / LAMPORTS_PER_SOL)
  });

  it("userB try to provide liqiuidity", async () => {
    const usdcAmount = 400 * USDC_DECIMALS;
    const solAmount = 8 * LAMPORTS_PER_SOL;

    const tx = await program.methods.deposit(new anchor.BN(usdcAmount),new anchor.BN(solAmount))
    .accountsPartial({
      signer: userB.publicKey, 
      usdcMint: usdc_mint, 
      wrappedSolMint: wrapped_solana_mint,
      userUsdcAta: userBUsdcAta, 
      userSolAta: userBSolAta,
      userPda: userBPda, 
      poolPda: pool_pda, 
      poolSolAta: poolSolAta, 
      poolUsdcAta: poolUsdcAta,
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .signers([userB])
    .rpc()
    const poolusdcAccountInfo = await getAccount(provider.connection, poolUsdcAta)
    const poolSolAtaInfo = await getAccount(provider.connection,poolSolAta)


    console.log("This is the balance  of the liquidity pool usdc after swapping", Number(poolusdcAccountInfo.amount) / USDC_DECIMALS)
    console.log("This is the balance of the liquidity pool solana after swapping", Number(poolSolAtaInfo.amount) / LAMPORTS_PER_SOL)
  })
  // it("userB try to provide liqiuidity with 1 percent change", async () => {
  //   const usdcAmount = 101 * USDC_DECIMALS;
  //   const solAmount = 2 * LAMPORTS_PER_SOL;

  //   const tx = await program.methods.deposit(new anchor.BN(usdcAmount),new anchor.BN(solAmount))
  //   .accountsPartial({
  //     signer: userB.publicKey, 
  //     usdcMint: usdc_mint, 
  //     wrappedSolMint: wrapped_solana_mint,
  //     userUsdcAta: userBUsdcAta, 
  //     userSolAta: userBSolAta,
  //     userPda: userBPda, 
  //     poolPda: pool_pda, 
  //     poolSolAta: poolSolAta, 
  //     poolUsdcAta: poolUsdcAta,
  //     tokenProgram: TOKEN_PROGRAM_ID
  //   })
  //   .signers([userB])
  //   .rpc()
  // })

  //  it("userB try to provide liqiuidity with  imabalance liquidity", async () => {
  //   //this test should fail 
  //   const usdcAmount = 102 * USDC_DECIMALS;
  //   const solAmount = 2 * LAMPORTS_PER_SOL;

  //   const tx = await program.methods.deposit(new anchor.BN(usdcAmount),new anchor.BN(solAmount))
  //   .accountsPartial({
  //     signer: userB.publicKey, 
  //     usdcMint: usdc_mint, 
  //     wrappedSolMint: wrapped_solana_mint,
  //     userUsdcAta: userBUsdcAta, 
  //     userSolAta: userBSolAta,
  //     userPda: userBPda, 
  //     poolPda: pool_pda, 
  //     poolSolAta: poolSolAta, 
  //     poolUsdcAta: poolUsdcAta,
  //     tokenProgram: TOKEN_PROGRAM_ID
  //   })
  //   .signers([userB])
  //   .rpc()
  // })

  it("swaping the usdc token with solana from the pool", async () => {
    const swapAmount = 500 * USDC_DECIMALS

    const tx = await program.methods.swap(new anchor.BN(swapAmount))
    .accountsPartial({
      signer: swapUser.publicKey, 
      usdcMint: usdc_mint, 
      wrappedSolMint: wrapped_solana_mint, 
      baseMint: wrapped_solana_mint, 
      poolUsdcAta: poolUsdcAta, 
      poolSolAta: poolSolAta, 
      userBaseAta: swapUserSolAta, 
      userQuoteAta: swapUserUsdcAta,
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .signers([swapUser])
    .rpc()

    const poolusdcAccountInfo = await getAccount(provider.connection, poolUsdcAta)
    const poolSolAtaInfo = await getAccount(provider.connection,poolSolAta)
    const poolPdaData = await program.account.pool.fetch(pool_pda)

    console.log("this is the data of the account after swapping", poolPdaData.feesCollectedUsdc.toNumber() / USDC_DECIMALS)

    console.log("This is the balance  of the liquidity pool usdc after swapping", Number(poolusdcAccountInfo.amount) / USDC_DECIMALS)
    console.log("This is the balance of the liquidity pool solana after swapping", Number(poolSolAtaInfo.amount) / LAMPORTS_PER_SOL)
  })

  it("userB withdraw amount", async () => {
    const tx = await program.methods.withdraw()
    .accountsPartial({
      signer: userB.publicKey, 
      usdcMint: usdc_mint, 
      wrappedSolMint: wrapped_solana_mint,
      userPda: userBPda,
      poolPda:pool_pda, 
      poolUsdcAta: poolUsdcAta, 
      poolWrappedSolAta: poolSolAta,
      userSolAta: userBSolAta, 
      userUsdcAta: userBUsdcAta, 
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .signers([userB])
    .rpc()

    const poolusdcAccountInfo = await getAccount(provider.connection, poolUsdcAta)
    const poolSolAtaInfo = await getAccount(provider.connection,poolSolAta)
    const poolPdaData = await program.account.pool.fetch(pool_pda)

    console.log("this is the data of the account after withdrawing", poolPdaData.feesCollectedUsdc.toNumber() / USDC_DECIMALS)


    console.log("This is the balance  of the liquidity pool usdc after swapping", Number(poolusdcAccountInfo.amount) / USDC_DECIMALS)
    console.log("This is the balance of the liquidity pool solana after swapping", Number(poolSolAtaInfo.amount) / LAMPORTS_PER_SOL)
  })

  // it("userB withdraw amount", async () => {
  //   const tx = await program.methods.withdraw()
  //   .accountsPartial({
  //     signer: userB.publicKey, 
  //     usdcMint: usdc_mint, 
  //     wrappedSolMint: wrapped_solana_mint,
  //     userPda: userBPda,
  //     poolPda:pool_pda, 
  //     poolUsdcAta: poolUsdcAta, 
  //     poolWrappedSolAta: poolSolAta,
  //     userSolAta: userBSolAta, 
  //     userUsdcAta: userBUsdcAta, 
  //     tokenProgram: TOKEN_PROGRAM_ID
  //   })
  //   .signers([userB])
  //   .rpc()
  // })

  it("userA withdraw amount", async () => {
    const tx = await program.methods.withdraw()
    .accountsPartial({
      signer: userA.publicKey,
      usdcMint: usdc_mint, 
      wrappedSolMint: wrapped_solana_mint ,
      userPda: userAPda,
      poolPda:pool_pda, 
      poolUsdcAta: poolUsdcAta, 
      poolWrappedSolAta: poolSolAta,
      userSolAta: userASolAta, 
      userUsdcAta: userAUsdcAta, 
      tokenProgram: TOKEN_PROGRAM_ID
    })
    .signers([userA])
    .rpc()

    const poolusdcAccountInfo = await getAccount(provider.connection, poolUsdcAta)
    const poolSolAtaInfo = await getAccount(provider.connection,poolSolAta)
    const poolPdaData = await program.account.pool.fetch(pool_pda)

    console.log("this is the data of the account usdc", poolPdaData.totalUsdcDeposit.toNumber() / USDC_DECIMALS)

    console.log("This is the balance of the liquidity pool usdc", Number(poolusdcAccountInfo.amount) / USDC_DECIMALS)
    console.log("This is the balance of the liquidity pool solana", Number(poolSolAtaInfo.amount) / LAMPORTS_PER_SOL)

  })

});
