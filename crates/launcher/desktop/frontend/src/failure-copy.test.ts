import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {Schema} from 'effect';
import {FailureCode} from './failure-codes';
import {failureCopy,failureText} from './failure-copy';

// The fixture the Rust tests read too (C-PD1).
const fixture=JSON.parse(readFileSync('src/failure-codes.json','utf8')) as
 {schema_version:number;codes:string[];job_errors:Record<string,string>;install_outcomes:Record<string,string>};
const isCode=Schema.is(FailureCode);

test('the FailureCode literals equal the fixture codes, in order',()=>{
 assert.equal(fixture.schema_version,1);
 assert.deepEqual([...FailureCode.literals],fixture.codes);
});

test('every code has a short message and fix',()=>{
 assert.deepEqual(Object.keys(failureCopy),fixture.codes);
 for(const code of fixture.codes){
  if(!isCode(code)) assert.fail(`${code} is not a FailureCode`);
  const {message,fix}=failureCopy[code];
  assert.ok(message.length>0&&message.length<=160,`${code} message`);
  assert.ok(fix.length>0&&fix.length<=200,`${code} fix`);
 }
});

test('only disk_space_low carries placeholders, and failureText fills them',()=>{
 for(const code of FailureCode.literals){
  const {message,fix}=failureCopy[code];
  assert.equal(/[{}]/.test(message+fix),code==='disk_space_low',code);
 }
 const {message}=failureText('disk_space_low',{needed:14e9,free:2.5e9});
 assert.ok(message.includes('about 14.0 GB is needed and 2.5 GB is free'),message);
 assert.ok(!/[{}]/.test(message),message);
});

test('every job_errors and install_outcomes value is a code',()=>{
 for(const value of [...Object.values(fixture.job_errors),...Object.values(fixture.install_outcomes)])
  assert.ok(isCode(value),value);
});
