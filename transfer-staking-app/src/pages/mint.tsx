import { Fragment, useRef, useState, useEffect } from 'react';
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import {
  Connection,
  Keypair,
  Signer,
  PublicKey,
  Transaction,
  TransactionSignature,
  ConfirmOptions,
  sendAndConfirmRawTransaction,
  RpcResponseAndContext,
  SimulatedTransactionResponse,
  Commitment,
  LAMPORTS_PER_SOL,
  SYSVAR_CLOCK_PUBKEY,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  clusterApiUrl
} from '@solana/web3.js'
import * as splToken from '@solana/spl-token'
import * as anchor from '@project-serum/anchor'
import useNotify from './notify'
const idl = require('./solana_anchor.json')
const programId = new PublicKey('2a7b125NsZNf4mkFvJJKH1JCUzTqNAEmzWvHUhrZhWrR')
const metadataProgramId = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s')

let wallet : any
let conn = new Connection(clusterApiUrl('devnet'))
let notify: any

export async function getAssociateTokenAddress(mint: any, owner: any) {
  let [address] = await PublicKey.findProgramAddress(
    [owner.toBuffer(), splToken.TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
  );
  return address;
}

async function mintNft(poolString : string ,data : any){
    if(!wallet) return '';
	try {
		let provider = new anchor.Provider(conn, wallet as any, anchor.Provider.defaultOptions())
	 	let program = new anchor.Program(idl,programId,provider)
	 	let pool = new PublicKey(poolString)
		let poolData = await program.account.pool.fetch(pool)

		let transaction = new Transaction();
	    let signers : Keypair[] = []
	    const mintRent = await conn.getMinimumBalanceForRentExemption(splToken.MintLayout.span)
	    const account = Keypair.generate()
	    const mint = account.publicKey;
	    signers.push(account)   
	    transaction.add(
	        SystemProgram.createAccount({
	            fromPubkey : wallet.publicKey,
	            newAccountPubkey : mint,
	            lamports : mintRent,
	            space : splToken.MintLayout.span,
	            programId : splToken.TOKEN_PROGRAM_ID
	        })
	    )
	    transaction.add(
	        await splToken.Token.createInitMintInstruction(
	            splToken.TOKEN_PROGRAM_ID,
	            mint,
	            0,
	            wallet.publicKey,
	            wallet.publicKey,
	        )
	    )
	    let ata = await getAssociateTokenAddress(mint, poolData.presaleOwner);
	    transaction.add(
	        await splToken.Token.createAssociatedTokenAccountInstruction(
	          splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
	          splToken.TOKEN_PROGRAM_ID,
	          mint,
	          ata,
	          poolData.presaleOwner,
	          wallet.publicKey,
	        ),
	    )
	    transaction.feePayer = wallet.publicKey;
	    transaction.recentBlockhash = (await conn.getRecentBlockhash('max')).blockhash;
	    transaction.setSigners(wallet.publicKey,...signers.map(s => s.publicKey));
	    transaction.partialSign(...signers)
	    const signedTransaction = await wallet.signTransaction(transaction);
	    let hash = await conn.sendRawTransaction(signedTransaction.serialize());
	    await conn.confirmTransaction(hash);
		notify('success', ' Successfully create mint for nft')
	    let metadata = (await PublicKey.findProgramAddress([Buffer.from('metadata'),metadataProgramId.toBuffer(),mint.toBuffer()],metadataProgramId))[0]
	    let master_endition = (await PublicKey.findProgramAddress([Buffer.from('metadata'),metadataProgramId.toBuffer(),mint.toBuffer(),Buffer.from('edition')],metadataProgramId))[0]
    
    	await program.rpc.mintNft(
    		data,
    		{
    			accounts:{
    				owner : wallet.publicKey,
    				pool : pool,
    				mint : mint,
    				tokenAccount : ata,
    				metadata : metadata,
    				masterEdition : master_endition,
	               	tokenMetadataProgram : metadataProgramId,
	                tokenProgram : splToken.TOKEN_PROGRAM_ID,
	                systemProgram : anchor.web3.SystemProgram.programId,
	                rent : SYSVAR_RENT_PUBKEY,
    			}
    		}
    	)
    	notify('success', ' Successfully mint nft')
    	return mint.toBase58()
    } catch(err) {
    	notify('error', ' Failed mint nft')
    	console.log(err)
    	return ''
    }
}

async function initPool(){
	try{
		const pool = Keypair.generate()
		const saleMint = new PublicKey('So11111111111111111111111111111111111111112')	
		let ata = await getAssociateTokenAddress(saleMint, wallet.publicKey)

		if((await conn.getAccountInfo(ata)) == null) {
		    let transaction = new Transaction()
		    transaction.add(
		        await splToken.Token.createAssociatedTokenAccountInstruction(
		          splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
		          splToken.TOKEN_PROGRAM_ID,
		          saleMint,
		          ata,
		          wallet.publicKey,
		          wallet.publicKey,
		        ),
		    );
		    transaction.feePayer = wallet.publicKey;
			transaction.recentBlockhash = (await conn.getRecentBlockhash('max')).blockhash;
			transaction.setSigners(wallet)
			const signedTransaction = await wallet.signTransaction(transaction);
		    let hash = await conn.sendRawTransaction(signedTransaction.serialize());
		    await conn.confirmTransaction(hash);
		    notify('success', ' Successfully generate token account')
		}
		
		let provider = new anchor.Provider(conn, wallet as any, anchor.Provider.defaultOptions())
		let program = new anchor.Program(idl,programId,provider)
		const [presaleOwner,bump] = await PublicKey.findProgramAddress([pool.publicKey.toBuffer(),wallet.publicKey.toBuffer()],programId)

		await program.rpc.initPool(
			new anchor.BN(bump),
			{
				accounts:{
					pool : pool.publicKey,
					owner : wallet.publicKey,
					presaleOwner : presaleOwner,
					saleMint : saleMint,
					poolWallet : ata,
					systemProgram : anchor.web3.SystemProgram.programId,
				},
				signers:[pool]
			}
		)
		notify('success', ' Successfully build pool')
		return pool.publicKey.toBase58()
	} catch(err) {
		notify('error', ' Failed build pool')
		console.log(err)
		return ''
	}
}

export default function Mint(){
	wallet = useWallet()
	notify = useNotify()
	// conn = useConnection()
	const [market,setMarket] = useState('')
	const [name,setName] = useState('')
	const [symbol,setSymbol] = useState('')
	const [uri,setUri] = useState('https://arweave.net/a03hkxJcMxG4DR-VtkE0WMMXL8-NWluV9-IU5RtMFKc')
	const [fee,setFee] = useState(300)
	const [count,setCount] = useState(0)
	const [mint,setMint] = useState('')
	return <div className="container-fluid mt-4">
		<div className="row">
			<div className="col-lg-6">
				<button type="button" className="btn btn-warning mb-3" onClick={async ()=>{
					setMarket(await initPool())
				}}>Create Market</button>
				<div className="input-group mb-3">
					<div className="input-group-prepend">
						<span className="input-group-text">Market Address</span>
					</div>
					<input name="nftMarketPlace" type="text" className="form-control" onChange={(event)=>{setMarket(event.target.value)}} value={market}/>
				</div>
				<div className="input-group mb-3">
					<div className="input-group-prepend">
						<span className="input-group-text">NFT Name</span>
					</div>
					<input name="nftName" type="text" className="form-control" onChange={(event)=>{setName(event.target.value)}} value={name}/>
				</div>
				<div className="input-group mb-3">
					<div className="input-group-prepend">
						<span className="input-group-text">Symbol</span>
					</div>
					<input name="nftSymbol" type="text" className="form-control" onChange={(event)=>{setSymbol(event.target.value)}} value={symbol}/>
				</div>
				<div className="input-group mb-3">
					<div className="input-group-prepend">
						<span className="input-group-text">NFT URI</span>
					</div>
					<input name="nftURI" type="text" className="form-control" onChange={(event)=>{setUri(event.target.value)}} value={uri}/>
				</div>
				<div className="input-group mb-3">
					<div className="input-group-prepend">
						<span className="input-group-text">Seller Fee</span>
					</div>
					<input name="nftFee" type="number" className="form-control" onChange={(event)=>{
						let val = Number(event.target.value)
						if(val > 10000) val = 10000
						setFee(val)}} value={fee}/>
				</div>
				
				<button type="button" className="btn btn-primary mb3" onClick={async ()=>{
					if(wallet && wallet.connected)
					setMint(await mintNft(market,{
						name : name,
						symbol : symbol,
						uri : uri,
						sellerFeeBasisPoints : fee,
						creators : [
							{address : wallet?.publicKey, verified : false, share : 100}
						],
						isMutable : true,
					}))

				}}>Mint</button>
				<div>{mint}</div>
			</div>
		</div>
	</div>
}