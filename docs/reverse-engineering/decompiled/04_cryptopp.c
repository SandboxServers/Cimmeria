/*
 * SGW.exe Decompilation - 04_cryptopp
 * Classes: 1
 * Stargate Worlds Client
 */


/* ========== CryptoPP.c ========== */

/*
 * SGW.exe - CryptoPP (22 functions)
 */

undefined4 * __thiscall CryptoPP_AlgorithmParametersBase__vfunc_0(void *this,byte param_1)

{
  bool bVar1;
  undefined4 auStack_2c [11];
  
  *(undefined ***)this = CryptoPP::AlgorithmParametersBase::vftable;
  bVar1 = std::uncaught_exception();
  if (((!bVar1) && (*(char *)((int)this + 8) != '\0')) && (*(char *)((int)this + 9) == '\0')) {
    FUN_00404ad0(auStack_2c);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(auStack_2c,(ThrowInfo *)&DAT_01d65a20);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


exception * __thiscall
CryptoPP_AlgorithmParametersBase_ParameterNotUsed__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CryptoPP::Exception::vftable;
  std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::
  ~basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>
            ((basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> *)
             ((int)this + 0x10));
  std::exception::~exception(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


int __thiscall CryptoPP_RandomPool__vfunc_0(void *this,byte param_1)

{
  ZipFileSystem__unknown_00406b30((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return (int)this;
}


void __thiscall
CryptoPP_CryptoPP_Rijndael_VEnc__0A___BlockCipherFinal__vfunc_0(void *this,byte param_1)

{
  FUN_00407030((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_HashVerificationFilter__vfunc_0(void *this,byte param_1)

{
  FUN_00409520((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_FilterWithBufferedInput__vfunc_0(void *this,byte param_1)

{
  FUN_00407ff0((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_ProxyFilter__vfunc_0(void *this,byte param_1)

{
  FUN_004087b0((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_StreamTransformationFilter__vfunc_0(void *this,byte param_1)

{
  FUN_00408b30((void *)((int)this + -4),param_1);
  return;
}


int __thiscall CryptoPP_SHA1__vfunc_0(void *this,byte param_1)

{
  if (*(void **)((int)this + 0xac) == (void *)((int)this + 0x60)) {
    *(undefined1 *)((int)this + 0xa1) = 0;
    memset(*(void **)((int)this + 0xac),0,*(int *)((int)this + 0xa8) * 4);
  }
  if (*(void **)((int)this + 0x5c) == (void *)((int)this + 0x10)) {
    *(undefined1 *)((int)this + 0x51) = 0;
    memset(*(void **)((int)this + 0x5c),0,*(int *)((int)this + 0x58) * 4);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return (int)this;
}


void * __thiscall CryptoPP_CombinedNameValuePairs__vfunc_0(void *this,byte param_1)

{
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


void __thiscall CryptoPP_ByteQueue__vfunc_0(void *this,byte param_1)

{
  FUN_0040e360((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_ArraySink__vfunc_0(void *this,byte param_1)

{
  CryptoPP_CombinedNameValuePairs__vfunc_0((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_MessageQueue__vfunc_0(void *this,byte param_1)

{
  FUN_00410840((void *)((int)this + -4),param_1);
  return;
}


void __thiscall CryptoPP_HashFilter__vfunc_0(void *this,byte param_1)

{
  FUN_00414800((void *)((int)this + -4),param_1);
  return;
}


undefined4 * __thiscall CryptoPP__N___AlgorithmParametersBase2__vfunc_0(void *this,byte param_1)

{
  FUN_00dd6020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
CryptoPP__N_CryptoPP_VNullNameValuePairs___AlgorithmParameters__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017054b8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_00dd6020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


int __thiscall CryptoPP_BaseN_Encoder__vfunc_0(void *this,byte param_1)

{
  FUN_00de2cb0((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return (int)this;
}


int __thiscall CryptoPP_Grouper__vfunc_0(void *this,byte param_1)

{
  FUN_00de2dd0((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return (int)this;
}


int __thiscall CryptoPP_SimpleProxyFilter__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01706168;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_00407a00((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return (int)this;
}


int __thiscall CryptoPP_HexEncoder__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017062d0;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_00407a00((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return (int)this;
}


int __thiscall CryptoPP_CryptoPP_Weak1_VMD5___HMAC__vfunc_0(void *this,byte param_1)

{
  FUN_01604c50((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return (int)this;
}


int __thiscall
CryptoPP_CryptoPP_Rijndael__00VDec___BlockCipherFinal__vfunc_0(void *this,byte param_1)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01798888;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_00407140((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar1);
  }
  ExceptionList = local_c;
  return (int)this;
}



