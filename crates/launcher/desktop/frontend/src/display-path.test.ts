import test from 'node:test';
import assert from 'node:assert/strict';
import {displayPath} from './display-path';

test('the extended-length prefix is removed from drive and UNC paths only',()=>{
  assert.equal(displayPath('\\\\?\\UNC\\server\\share\\SGW'),'\\\\server\\share\\SGW');
  assert.equal(displayPath('\\\\?\\C:\\Games\\SGW'),'C:\\Games\\SGW');
  assert.equal(displayPath('C:\\Games\\SGW'),'C:\\Games\\SGW');
  assert.equal(displayPath('\\\\server\\share\\SGW'),'\\\\server\\share\\SGW');
  assert.equal(displayPath('/games/SGW'),'/games/SGW');
  assert.equal(displayPath('\\\\?\\Volume{1234}\\SGW'),'\\\\?\\Volume{1234}\\SGW');
});
