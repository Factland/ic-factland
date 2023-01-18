import fetch from 'node-fetch';
import fs from 'fs';
import crypto from 'crypto';
import sha256 from "sha256";
import { lebDecode, PipeArrayBuffer } from "@dfinity/candid";
import { Principal } from '@dfinity/principal';
import { Secp256k1PublicKey, Secp256k1KeyIdentity } from '@dfinity/identity';
import { Actor, Cbor, Certificate, HttpAgent, lookup_path, reconstruct, hashTreeToString } from '@dfinity/agent';
import { idlFactory } from '../src/declarations/factland/factland.did.js';
import exec from 'await-exec';
import assert from 'assert';

function toHex(buffer) { // buffer is an ArrayBuffer
	return [...new Uint8Array(buffer)]
		.map(x => x.toString(16).padStart(2, '0'))
		.join('');
}

function fromHex(hex) {
	const hexRe = new RegExp(/^([0-9A-F]{2})*$/i);
	if (!hexRe.test(hex)) {
		throw new Error("Invalid hexadecimal string.");
	}
	const buffer = [...hex]
		.reduce((acc, curr, i) => {
			acc[(i / 2) | 0] = (acc[(i / 2) | 0] || "") + curr;
			return acc;
		}, [])
		.map((x) => Number.parseInt(x, 16));

	return new Uint8Array(buffer).buffer;
}

function mergeUInt8Arrays(a1, a2) {
  var mergedArray = new Uint8Array(a1.length + a2.length);
  mergedArray.set(a1);
  mergedArray.set(a2, a1.length);
  return mergedArray;
}

function isBufferEqual(a, b) {
  if (a.byteLength !== b.byteLength) {
		return false;
	}
	const a8 = new Uint8Array(a);
	const b8 = new Uint8Array(b);
	for (let i = 0; i < a8.length; i++) {
		if (a8[i] !== b8[i]) {
			return false;
		}
	}
	return true;
}

function blockToHex(block) {
	return {
		certificate: toHex(block.certificate),
		tree: toHex(block.tree),
		data: block.data.map((x) => toHex(x)),
		previous_hash: toHex(block.previous_hash)
	};
}

// Install the global brower compatible fetch.
global.fetch = fetch;

// Obtain identity.
const privateKeyFile = fs.readFileSync('/home/jplevyak/.config/dfx/identity/factland/identity.pem');
const privateKeyObject = crypto.createPrivateKey({
	key: privateKeyFile,
	format: 'pem'
});
const privateKeyDER = privateKeyObject.export({
	format: 'der',
	type: 'sec1',
});
const PEM_DER_PREFIX = new Uint8Array([0x30, 0x74, 0x02, 0x01, 0x01, 0x04, 0x20]);
assert(isBufferEqual(PEM_DER_PREFIX, privateKeyDER.slice(0, 7)));
let secret_key = new Uint8Array(privateKeyDER.slice(7, 7+32));
const identity = Secp256k1KeyIdentity.fromSecretKey(secret_key);
const principal = identity.getPrincipal().toText();
const raw_principal = identity.getPrincipal().toUint8Array();

// Authorize this identity.
console.log('authorizing principal', principal);
let authorize_cmd = 'dfx canister --network iccall factland stable_authorize \'(principal "' + principal + '")\'';
console.log('exec:', authorize_cmd, await exec(authorize_cmd));

//const canisterId = "rrkah-fqaaa-aaaaa-aaaaq-cai";
//const url = 'http://localhost:8080';
const canisterId = "5u3nb-maaaa-aaaae-qaega-cai";
const url = 'https://ic0.app';

export const createActor = (idlFactory, canisterId, options) => {
	let agentOptions = options ? {...options.agentOptions} : {};
	const agent = new HttpAgent(agentOptions);
  /*
  agent.fetchRootKey().catch(err => {
    console.warn('Unable to fetch root key. Check to ensure that your local replica is running');
    console.error(err);
  });
  */
	return Actor.createActor(idlFactory, {
		agent,
		canisterId,
		...(options ? options.actorOptions : {}),
	});
};

let actor = createActor(idlFactory, canisterId, { agentOptions: { host: url, identity }});

BigInt.prototype.toJSON = function() { return Number(this); };
let fd = fs.openSync("stable_memory.dat", "r");
let stats = fs.fstatSync(fd);
let size = stats.size;
console.log('stable_size', size);
let fd_old = 0;
if (fs.existsSync("stable_memory.dat.orig")) {
  fd_old = fs.openSync("stable_memory.dat.orig", "r");
}
let BLOCK_SIZE = 1024 * 1024;
let blocks = [];
let offset = 0;
while (size > 0) {
  let s = size;
  if (s > BLOCK_SIZE) {
    s = BLOCK_SIZE
  }
  console.log('stable_write', 0, s, offset);
  let b = Buffer.alloc(s);
  fs.readSync(fd, b);
  if (fd_old != 0) {
    let b_old = Buffer.alloc(s);
    fs.readSync(fd_old, b_old);
    if (isBufferEqual(b, b_old)) {
      console.log('same as stable_memory.dat.orig');
      size -= s;
      offset += s;
      continue;
    }
  }
  await actor.stable_write(offset, b);
  size -= s;
  offset += s;
}