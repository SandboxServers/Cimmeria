/*
 * SGW.exe Decompilation - 02_bigworld_network
 * Classes: 114
 * Stargate Worlds Client
 */


/* ========== ABigWorldEntity.c ========== */

/*
 * SGW.exe - ABigWorldEntity (3 functions)
 */

/* [VTable] ABigWorldEntity virtual function #2
   VTable: vtable_ABigWorldEntity at 01895e3c */

int * __thiscall ABigWorldEntity__vfunc_2(void *this,byte param_1)

{
  FUN_0077ed20(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* Also in vtable: ABigWorldEntity (slot 0) */

undefined4 ABigWorldEntity__vfunc_0(void)

{
  return 1;
}


/* [VTable] ABigWorldEntity virtual function #1
   VTable: vtable_ABigWorldEntity at 01895e3c */

bool __fastcall ABigWorldEntity__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee3998 == (undefined4 *)0x0) {
    DAT_01ee3998 = FUN_0076f700();
    FUN_00769330();
  }
  return puVar1 != DAT_01ee3998;
}




/* ========== ArrayDataType.c ========== */

/*
 * SGW.exe - ArrayDataType (1 functions)
 */

undefined4 * __thiscall ArrayDataType__vfunc_0(void *this,byte param_1)

{
  FUN_01598fa0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BW.c ========== */

/*
 * SGW.exe - BW (1 functions)
 */

undefined * BW__unknown_0155f790(void)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  puStack_8 = &LAB_01790f0c;
  pvStack_c = ExceptionList;
  if ((DAT_01f11fd0 & 1) == 0) {
    DAT_01f11fd0 = DAT_01f11fd0 | 1;
    uStack_4 = 0;
    ExceptionList = &pvStack_c;
    FUN_00a5c1d0(0x1f11fc4);
    uStack_4 = uStack_4 & 0xffffff00;
    FUN_012375cb((_onexit_t)&LAB_017ede50);
  }
  ExceptionList = pvStack_c;
  return &DAT_01f11fc4;
}




/* ========== BWChunk.c ========== */

/*
 * SGW.exe - BWChunk (1 functions)
 */

void __thiscall
BWChunk__vfunc_0(void *this,undefined4 param_1,undefined4 param_2,float param_3,int param_4,
                int param_5,int param_6)

{
  ulonglong uVar1;
  int *piVar2;
  undefined8 *puVar3;
  void *pvVar4;
  float local_4c;
  float fStack_48;
  float local_44;
  float fStack_40;
  int local_3c [3];
  ulonglong local_30;
  float local_28;
  int local_24 [4];
  void *local_14;
  undefined1 *puStack_10;
  int local_c;
  
  pvVar4 = ExceptionList;
  local_44 = DAT_01814190;
  local_c = 0xffffffff;
  puStack_10 = &LAB_0173cc13;
  local_14 = ExceptionList;
  if (param_6 == 0) {
    local_4c = DAT_017f7ea0;
    fStack_48 = 0.0;
    local_44 = 0.0;
    fStack_40 = 0.0;
    ExceptionList = &local_14;
    *(ulonglong *)((int)this + 8) = (ulonglong)(uint)DAT_017f7ea0;
    *(undefined8 *)((int)this + 0x10) = 0;
    if ((*(byte *)((int)this + 0xe0) & 1) != 0) {
      ExceptionList = pvVar4;
      return;
    }
    BWChunk__unknown_0105b6c0(local_3c);
    local_c = 0;
    piVar2 = BWChunk__unknown_010538c0(&local_4c,param_4 + -1,param_5);
    local_c._0_1_ = 1;
    FUN_0041a630(local_3c,piVar2);
    local_c = (uint)local_c._1_3_ << 8;
    FUN_0041b420((int *)&local_4c);
    local_30 = (ulonglong)(uint)DAT_017f7ea0 << 0x20;
    local_4c = *(float *)((int)this + 0xb4);
    local_28 = 0.0;
    fStack_48 = 0.0;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    local_4c = *(float *)((int)this + 0xc0);
    fStack_48 = 0.0;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    local_4c = *(float *)((int)this + 0xc0);
    fStack_48 = param_3;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    local_4c = *(float *)((int)this + 0xb4);
    fStack_48 = param_3;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    pvVar4 = (void *)((int)this + 0x18);
  }
  else if (param_6 == 1) {
    fStack_40 = DAT_01816a90 - param_3;
    local_4c = DAT_01814190;
    fStack_48 = 0.0;
    ExceptionList = &local_14;
    *(ulonglong *)((int)this + 0x24) = (ulonglong)(uint)DAT_01814190;
    local_44 = 0.0;
    *(ulonglong *)((int)this + 0x2c) = (ulonglong)(uint)fStack_40 << 0x20;
    if ((*(byte *)((int)this + 0xe0) & 2) != 0) {
      ExceptionList = pvVar4;
      return;
    }
    BWChunk__unknown_0105b6c0(local_3c);
    local_c = 2;
    piVar2 = BWChunk__unknown_010538c0(&local_4c,param_4 + 1,param_5);
    local_c._0_1_ = 3;
    FUN_0041a630(local_3c,piVar2);
    local_c = CONCAT31(local_c._1_3_,2);
    FUN_0041b420((int *)&local_4c);
    local_30 = 0;
    fStack_48 = *(float *)((int)this + 0xb4);
    local_28 = DAT_017f7ea0;
    local_4c = 0.0;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    fStack_48 = *(float *)((int)this + 0xb4);
    local_4c = param_3;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    fStack_48 = *(float *)((int)this + 0xc0);
    local_4c = param_3;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    fStack_48 = *(float *)((int)this + 0xc0);
    local_4c = 0.0;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    pvVar4 = (void *)((int)this + 0x34);
  }
  else if (param_6 == 2) {
    local_4c = 0.0;
    fStack_40 = *(float *)((int)this + 0xb4);
    fStack_48 = DAT_017f7ea0;
    local_44 = 0.0;
    ExceptionList = &local_14;
    *(ulonglong *)((int)this + 0x40) = (ulonglong)(uint)DAT_017f7ea0 << 0x20;
    *(ulonglong *)((int)this + 0x48) = (ulonglong)(uint)fStack_40 << 0x20;
    BWChunk__unknown_0105b6c0(local_3c);
    local_c = 4;
    FUN_0047d9d0(&local_4c,"earth");
    local_c._0_1_ = 5;
    FUN_0041a630(local_3c,(int *)&local_4c);
    local_c = CONCAT31(local_c._1_3_,4);
    FUN_0041b420((int *)&local_4c);
    local_30 = 0;
    local_28 = DAT_017f7ea0;
    local_4c = 0.0;
    fStack_48 = 0.0;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    local_4c = param_3;
    fStack_48 = 0.0;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    local_4c = param_3;
    fStack_48 = param_3;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    local_4c = 0.0;
    fStack_48 = param_3;
    local_44 = 0.0;
    puVar3 = (undefined8 *)BWChunk__unknown_004fc210(0xc,local_24);
    if (puVar3 != (undefined8 *)0x0) {
      *puVar3 = CONCAT44(fStack_48,local_4c);
      *(float *)(puVar3 + 1) = local_44;
    }
    pvVar4 = (void *)((int)this + 0x50);
  }
  else if (param_6 == 3) {
    local_4c = 0.0;
    fStack_40 = DAT_01816a90 - *(float *)((int)this + 0xc0);
    fStack_48 = DAT_01814190;
    local_44 = 0.0;
    ExceptionList = &local_14;
    *(ulonglong *)((int)this + 0x5c) = (ulonglong)(uint)DAT_01814190 << 0x20;
    *(ulonglong *)((int)this + 100) = (ulonglong)(uint)fStack_40 << 0x20;
    BWChunk__unknown_0105b6c0(local_3c);
    local_c = 6;
    FUN_0047d9d0(&local_4c,"heaven");
    local_c._0_1_ = 7;
    FUN_0041a630(local_3c,(int *)&local_4c);
    local_c = CONCAT31(local_c._1_3_,6);
    FUN_0041b420((int *)&local_4c);
    local_30 = (ulonglong)(uint)DAT_017f7ea0;
    local_28 = 0.0;
    local_4c = 0.0;
    fStack_48 = 0.0;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    local_4c = param_3;
    fStack_48 = 0.0;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    local_4c = param_3;
    fStack_48 = param_3;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    local_4c = 0.0;
    fStack_48 = param_3;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    pvVar4 = (void *)((int)this + 0x6c);
  }
  else if (param_6 == 4) {
    local_4c = 0.0;
    fStack_48 = 0.0;
    local_44 = DAT_017f7ea0;
    fStack_40 = 0.0;
    uVar1 = (ulonglong)(uint)DAT_017f7ea0;
    ExceptionList = &local_14;
    *(undefined8 *)((int)this + 0x78) = 0;
    *(ulonglong *)((int)this + 0x80) = uVar1;
    if ((*(byte *)((int)this + 0xe0) & 4) != 0) {
      ExceptionList = pvVar4;
      return;
    }
    BWChunk__unknown_0105b6c0(local_3c);
    local_c = 8;
    piVar2 = BWChunk__unknown_010538c0(&local_4c,param_4,param_5 + -1);
    local_c._0_1_ = 9;
    FUN_0041a630(local_3c,piVar2);
    local_c = CONCAT31(local_c._1_3_,8);
    FUN_0041b420((int *)&local_4c);
    local_30 = (ulonglong)(uint)DAT_017f7ea0;
    fStack_48 = *(float *)((int)this + 0xb4);
    local_28 = 0.0;
    local_4c = 0.0;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    fStack_48 = *(float *)((int)this + 0xb4);
    local_4c = param_3;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    fStack_48 = *(float *)((int)this + 0xc0);
    local_4c = param_3;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    fStack_48 = *(float *)((int)this + 0xc0);
    local_4c = 0.0;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    pvVar4 = (void *)((int)this + 0x88);
  }
  else {
    if (param_6 != 5) {
      return;
    }
    fStack_40 = DAT_01816a90 - param_3;
    local_4c = 0.0;
    fStack_48 = 0.0;
    ExceptionList = &local_14;
    *(undefined8 *)((int)this + 0x94) = 0;
    *(ulonglong *)((int)this + 0x9c) = CONCAT44(fStack_40,local_44);
    if ((*(byte *)((int)this + 0xe0) & 8) != 0) {
      ExceptionList = pvVar4;
      return;
    }
    BWChunk__unknown_0105b6c0(local_3c);
    local_c = 10;
    piVar2 = BWChunk__unknown_010538c0(&local_4c,param_4,param_5 + 1);
    local_c._0_1_ = 0xb;
    FUN_0041a630(local_3c,piVar2);
    local_c = CONCAT31(local_c._1_3_,10);
    FUN_0041b420((int *)&local_4c);
    local_30 = (ulonglong)(uint)DAT_017f7ea0 << 0x20;
    local_4c = *(float *)((int)this + 0xb4);
    local_28 = 0.0;
    fStack_48 = 0.0;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    local_4c = *(float *)((int)this + 0xc0);
    fStack_48 = 0.0;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    local_4c = *(float *)((int)this + 0xc0);
    fStack_48 = param_3;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    local_4c = *(float *)((int)this + 0xb4);
    fStack_48 = param_3;
    local_44 = 0.0;
    BWChunk__unknown_004fc2a0(local_24,(undefined8 *)&local_4c);
    pvVar4 = (void *)((int)this + 0xa4);
  }
  FUN_0105bad0(pvVar4,local_3c);
  local_c = 0xffffffff;
  FUN_0105b720(local_3c);
  ExceptionList = local_14;
  return;
}




/* ========== BaseTask.c ========== */

/*
 * SGW.exe - BaseTask (1 functions)
 */

/* Also in vtable: detail__RootTask (slot 0) */

undefined4 * __thiscall BaseTask__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01688eb8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = tbb::task::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== BigWorld_Network.c ========== */

/*
 * SGW.exe - BigWorld_Network (2 functions)
 */

undefined4 * __thiscall BaseAppLoginHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00de4d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall BaseAppearanceJob__vfunc_0(void *this,byte param_1)

{
  FUN_00eb7590(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BinaryBlock.c ========== */

/*
 * SGW.exe - BinaryBlock (1 functions)
 */

/* Also in vtable: BinaryBlock (slot 0) */

undefined4 * __thiscall BinaryBlock__vfunc_0(void *this,byte param_1)

{
  FUN_004724d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BinaryIStream.c ========== */

/*
 * SGW.exe - BinaryIStream (1 functions)
 */

undefined4 * __thiscall BinaryIStream__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = BinaryIStream::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BinaryOStream.c ========== */

/*
 * SGW.exe - BinaryOStream (1 functions)
 */

undefined4 * __thiscall BinaryOStream__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = BinaryOStream::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BlobDataType.c ========== */

/*
 * SGW.exe - BlobDataType (1 functions)
 */

undefined4 * __thiscall BlobDataType__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01795f30;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_01595b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== CanSmoothBinormalsChecker.c ========== */

/*
 * SGW.exe - CanSmoothBinormalsChecker (1 functions)
 */

undefined4 CanSmoothBinormalsChecker__vfunc_0(int param_1,int param_2,float *param_3)

{
  undefined2 uVar1;
  float fVar2;
  float fVar3;
  float fVar4;
  undefined4 local_18;
  undefined4 uStack_14;
  undefined4 local_c;
  undefined4 uStack_8;
  
  fVar4 = *(float *)(param_1 + 0x2c);
  uStack_14 = (float)((ulonglong)*(undefined8 *)(param_1 + 0x24) >> 0x20);
  local_18 = (float)*(undefined8 *)(param_1 + 0x24);
  fVar3 = *(float *)(param_2 + 0x2c);
  uStack_8 = (float)((ulonglong)*(undefined8 *)(param_2 + 0x24) >> 0x20);
  fVar2 = DAT_017f7ea0 / SQRT(uStack_14 * uStack_14 + local_18 * local_18 + fVar4 * fVar4);
  local_c = (float)*(undefined8 *)(param_2 + 0x24);
  local_18 = local_18 * fVar2;
  uStack_14 = uStack_14 * fVar2;
  fVar2 = fVar2 * fVar4;
  fVar4 = DAT_017f7ea0 / SQRT(fVar3 * fVar3 + local_c * local_c + uStack_8 * uStack_8);
  fVar3 = fVar3 * fVar4;
  local_c = local_c * fVar4;
  fVar4 = fVar4 * uStack_8;
  if (local_c * local_18 + fVar3 * fVar2 + uStack_14 * fVar4 < *param_3) {
    uVar1 = (undefined2)((uint)param_2 >> 0x10);
    param_2 = (uint)CONCAT21(uVar1,(local_18 == 0.0) << 6 | NAN(local_18) << 2 | 2U | local_18 < 0.0
                            ) << 8;
    if ((((local_18 != 0.0) ||
         (param_2 = (uint)CONCAT21(uVar1,(uStack_14 == 0.0) << 6 | NAN(uStack_14) << 2 | 2U |
                                         uStack_14 < 0.0) << 8, uStack_14 != 0.0)) ||
        (param_2 = (uint)CONCAT21(uVar1,(fVar2 == 0.0) << 6 | NAN(fVar2) << 2 | 2U | fVar2 < 0.0) <<
                   8, fVar2 != 0.0)) ||
       (((param_2 = (uint)CONCAT21(uVar1,(local_c == 0.0) << 6 | NAN(local_c) << 2 | 2U |
                                         local_c < 0.0) << 8, local_c != 0.0 ||
         (param_2 = (uint)CONCAT21(uVar1,(fVar4 == 0.0) << 6 | NAN(fVar4) << 2 | 2U | fVar4 < 0.0)
                    << 8, fVar4 != 0.0)) ||
        (param_2 = (uint)CONCAT21(uVar1,(fVar3 == 0.0) << 6 | NAN(fVar3) << 2 | 2U | fVar3 < 0.0) <<
                   8, fVar3 != 0.0)))) {
      return param_2;
    }
  }
  return CONCAT31((int3)((uint)param_2 >> 8),1);
}




/* ========== CanSmoothNormalsChecker.c ========== */

/*
 * SGW.exe - CanSmoothNormalsChecker (1 functions)
 */

undefined4 CanSmoothNormalsChecker__vfunc_0(int param_1,int param_2,float *param_3)

{
  undefined2 uVar1;
  float fVar2;
  float fVar3;
  float fVar4;
  undefined4 local_18;
  undefined4 uStack_14;
  undefined4 local_c;
  undefined4 uStack_8;
  
  fVar4 = *(float *)(param_1 + 0x14);
  uStack_14 = (float)((ulonglong)*(undefined8 *)(param_1 + 0xc) >> 0x20);
  local_18 = (float)*(undefined8 *)(param_1 + 0xc);
  fVar3 = *(float *)(param_2 + 0x14);
  uStack_8 = (float)((ulonglong)*(undefined8 *)(param_2 + 0xc) >> 0x20);
  fVar2 = DAT_017f7ea0 / SQRT(uStack_14 * uStack_14 + local_18 * local_18 + fVar4 * fVar4);
  local_c = (float)*(undefined8 *)(param_2 + 0xc);
  local_18 = local_18 * fVar2;
  uStack_14 = uStack_14 * fVar2;
  fVar2 = fVar2 * fVar4;
  fVar4 = DAT_017f7ea0 / SQRT(fVar3 * fVar3 + local_c * local_c + uStack_8 * uStack_8);
  fVar3 = fVar3 * fVar4;
  local_c = local_c * fVar4;
  fVar4 = fVar4 * uStack_8;
  if (local_c * local_18 + fVar3 * fVar2 + uStack_14 * fVar4 < *param_3) {
    uVar1 = (undefined2)((uint)param_2 >> 0x10);
    param_2 = (uint)CONCAT21(uVar1,(local_18 == 0.0) << 6 | NAN(local_18) << 2 | 2U | local_18 < 0.0
                            ) << 8;
    if ((((local_18 != 0.0) ||
         (param_2 = (uint)CONCAT21(uVar1,(uStack_14 == 0.0) << 6 | NAN(uStack_14) << 2 | 2U |
                                         uStack_14 < 0.0) << 8, uStack_14 != 0.0)) ||
        (param_2 = (uint)CONCAT21(uVar1,(fVar2 == 0.0) << 6 | NAN(fVar2) << 2 | 2U | fVar2 < 0.0) <<
                   8, fVar2 != 0.0)) ||
       (((param_2 = (uint)CONCAT21(uVar1,(local_c == 0.0) << 6 | NAN(local_c) << 2 | 2U |
                                         local_c < 0.0) << 8, local_c != 0.0 ||
         (param_2 = (uint)CONCAT21(uVar1,(fVar4 == 0.0) << 6 | NAN(fVar4) << 2 | 2U | fVar4 < 0.0)
                    << 8, fVar4 != 0.0)) ||
        (param_2 = (uint)CONCAT21(uVar1,(fVar3 == 0.0) << 6 | NAN(fVar3) << 2 | 2U | fVar3 < 0.0) <<
                   8, fVar3 != 0.0)))) {
      return param_2;
    }
  }
  return CONCAT31((int3)((uint)param_2 >> 8),1);
}




/* ========== CanSmoothTangentsChecker.c ========== */

/*
 * SGW.exe - CanSmoothTangentsChecker (1 functions)
 */

undefined4 CanSmoothTangentsChecker__vfunc_0(int param_1,int param_2,float *param_3)

{
  undefined2 uVar1;
  float fVar2;
  float fVar3;
  float fVar4;
  undefined4 local_18;
  undefined4 uStack_14;
  undefined4 local_c;
  undefined4 uStack_8;
  
  fVar4 = *(float *)(param_1 + 0x20);
  uStack_14 = (float)((ulonglong)*(undefined8 *)(param_1 + 0x18) >> 0x20);
  local_18 = (float)*(undefined8 *)(param_1 + 0x18);
  fVar3 = *(float *)(param_2 + 0x20);
  uStack_8 = (float)((ulonglong)*(undefined8 *)(param_2 + 0x18) >> 0x20);
  fVar2 = DAT_017f7ea0 / SQRT(uStack_14 * uStack_14 + local_18 * local_18 + fVar4 * fVar4);
  local_c = (float)*(undefined8 *)(param_2 + 0x18);
  local_18 = local_18 * fVar2;
  uStack_14 = uStack_14 * fVar2;
  fVar2 = fVar2 * fVar4;
  fVar4 = DAT_017f7ea0 / SQRT(fVar3 * fVar3 + local_c * local_c + uStack_8 * uStack_8);
  fVar3 = fVar3 * fVar4;
  local_c = local_c * fVar4;
  fVar4 = fVar4 * uStack_8;
  if (local_c * local_18 + fVar3 * fVar2 + uStack_14 * fVar4 < *param_3) {
    uVar1 = (undefined2)((uint)param_2 >> 0x10);
    param_2 = (uint)CONCAT21(uVar1,(local_18 == 0.0) << 6 | NAN(local_18) << 2 | 2U | local_18 < 0.0
                            ) << 8;
    if ((((local_18 != 0.0) ||
         (param_2 = (uint)CONCAT21(uVar1,(uStack_14 == 0.0) << 6 | NAN(uStack_14) << 2 | 2U |
                                         uStack_14 < 0.0) << 8, uStack_14 != 0.0)) ||
        (param_2 = (uint)CONCAT21(uVar1,(fVar2 == 0.0) << 6 | NAN(fVar2) << 2 | 2U | fVar2 < 0.0) <<
                   8, fVar2 != 0.0)) ||
       (((param_2 = (uint)CONCAT21(uVar1,(local_c == 0.0) << 6 | NAN(local_c) << 2 | 2U |
                                         local_c < 0.0) << 8, local_c != 0.0 ||
         (param_2 = (uint)CONCAT21(uVar1,(fVar4 == 0.0) << 6 | NAN(fVar4) << 2 | 2U | fVar4 < 0.0)
                    << 8, fVar4 != 0.0)) ||
        (param_2 = (uint)CONCAT21(uVar1,(fVar3 == 0.0) << 6 | NAN(fVar3) << 2 | 2U | fVar3 < 0.0) <<
                   8, fVar3 != 0.0)))) {
      return param_2;
    }
  }
  return CONCAT31((int3)((uint)param_2 >> 8),1);
}




/* ========== ClassDataType.c ========== */

/*
 * SGW.exe - ClassDataType (1 functions)
 */

undefined4 * __thiscall ClassDataType__vfunc_0(void *this,byte param_1)

{
  FUN_015a1050(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ClassMetaDataType.c ========== */

/*
 * SGW.exe - ClassMetaDataType (1 functions)
 */

undefined4 * __thiscall ClassMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_01598260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CollisionVisitor.c ========== */

/*
 * SGW.exe - CollisionVisitor (1 functions)
 */

undefined4 * __thiscall CollisionVisitor__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CollisionVisitor::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CreateMessage.c ========== */

/*
 * SGW.exe - CreateMessage (1 functions)
 */

undefined4 * __thiscall CreateMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01586630(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DataHandle.c ========== */

/*
 * SGW.exe - DataHandle (1 functions)
 */

undefined4 * __thiscall DataHandle__vfunc_0(void *this,byte param_1)

{
  FUN_0155cad0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DataResource.c ========== */

/*
 * SGW.exe - DataResource (1 functions)
 */

undefined4 * __thiscall DataResource__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0175f79b;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_004585a0((int *)((int)this + 8));
  local_4 = 0xffffffff;
  *(undefined ***)this = ReferenceCount::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== DataSection.c ========== */

/*
 * SGW.exe - DataSection (1 functions)
 */

/* Also in vtable: DataSection (slot 0) */

undefined4 * __thiscall DataSection__vfunc_0(void *this,byte param_1)

{
  FUN_00466ae0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Detail.c ========== */

/*
 * SGW.exe - Detail (2 functions)
 */

/* Also in vtable:
   CME_TypeList_0_0_0_0_0_0_0_0_0_0_0_0_0_UNullType_VVector4_U__TypeList_VVector3_U__TypeList_VVector2_U__TypeList_std_tbb__W_V__scalable_allocator_std__W__WU__char_traits_V__basic_string_MU__TypeList__JU__TypeList__KU__TypeList_JU__TypeList_KU__TypeList_FU__TypeList_GU__TypeList_DU__TypeList_EU__TypeList_U__TypeList___BasicPropertyList
   (slot 0) */

undefined4 * __thiscall
CME_TypeList_0_0_0_0_0_0_0_0_0_0_0_0_0_UNullType_VVector4_U__TypeList_VVector3_U__TypeList_VVector2_U__TypeList_std_tbb__W_V__scalable_allocator_std__W__WU__char_traits_V__basic_string_MU__TypeList__JU__TypeList__KU__TypeList_JU__TypeList_KU__TypeList_FU__TypeList_GU__TypeList_DU__TypeList_EU__TypeList_U__TypeList___BasicPropertyList__vfunc_0
          (void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01679e9b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this =
       CME::
       BasicPropertyList<struct_TypeList::TypeList<unsigned_char,struct_TypeList::TypeList<char,struct_TypeList::TypeList<unsigned_short,struct_TypeList::TypeList<short,struct_TypeList::TypeList<unsigned_long,struct_TypeList::TypeList<long,struct_TypeList::TypeList<unsigned___int64,struct_TypeList::TypeList<__int64,struct_TypeList::TypeList<float,struct_TypeList::TypeList<class_std::basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_tbb::scalable_allocator<wchar_t>_>,struct_TypeList::TypeList<class_Vector2,struct_TypeList::TypeList<class_Vector3,struct_TypeList::TypeList<class_Vector4,struct_TypeList::NullType>_>_>_>_>_>_>_>_>_>_>_>_>_>
       ::vftable;
  local_4 = 0xffffffff;
  Detail__unknown_0043d770((uint *)((int)this + 4));
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


/* Also in vtable:
   CME_TypeList_0_0_0_0_0_0_0_0_0_0_0_0_0_UNullType_VVector4_U__TypeList_VVector3_U__TypeList_VVector2_U__TypeList_std_tbb__W_V__scalable_allocator_std__W__WU__char_traits_V__basic_string_MU__TypeList__JU__TypeList__KU__TypeList_JU__TypeList_KU__TypeList_FU__TypeList_GU__TypeList_DU__TypeList_EU__TypeList_U__TypeList___BasicPropertyTree
   (slot 0) */

undefined4 * __thiscall
CME_TypeList_0_0_0_0_0_0_0_0_0_0_0_0_0_UNullType_VVector4_U__TypeList_VVector3_U__TypeList_VVector2_U__TypeList_std_tbb__W_V__scalable_allocator_std__W__WU__char_traits_V__basic_string_MU__TypeList__JU__TypeList__KU__TypeList_JU__TypeList_KU__TypeList_FU__TypeList_GU__TypeList_DU__TypeList_EU__TypeList_U__TypeList___BasicPropertyTree__vfunc_0
          (void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0168f42b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this =
       CME::
       BasicPropertyTree<struct_TypeList::TypeList<unsigned_char,struct_TypeList::TypeList<char,struct_TypeList::TypeList<unsigned_short,struct_TypeList::TypeList<short,struct_TypeList::TypeList<unsigned_long,struct_TypeList::TypeList<long,struct_TypeList::TypeList<unsigned___int64,struct_TypeList::TypeList<__int64,struct_TypeList::TypeList<float,struct_TypeList::TypeList<class_std::basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_tbb::scalable_allocator<wchar_t>_>,struct_TypeList::TypeList<class_Vector2,struct_TypeList::TypeList<class_Vector3,struct_TypeList::TypeList<class_Vector4,struct_TypeList::NullType>_>_>_>_>_>_>_>_>_>_>_>_>_>
       ::vftable;
  local_4 = 0xffffffff;
  Detail__unknown_00440a50((uint *)((int)this + 4));
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== Detail_BasePlayerCreateMessage.c ========== */

/*
 * SGW.exe - Detail_BasePlayerCreateMessage (1 functions)
 */

undefined4 * __thiscall Detail_BasePlayerCreateMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017913d3;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_00dd3d10((undefined4 *)((int)this + 0xc));
  local_4 = 0xffffffff;
  *(undefined ***)this = ISGWMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== Detail_CellPlayerCreateMessage.c ========== */

/*
 * SGW.exe - Detail_CellPlayerCreateMessage (1 functions)
 */

undefined4 * __thiscall Detail_CellPlayerCreateMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01791393;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_00dd3d10((undefined4 *)((int)this + 0x28));
  local_4 = 0xffffffff;
  *(undefined ***)this = ISGWMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== Detail_EnableEntitiesRejectedMessage.c ========== */

/*
 * SGW.exe - Detail_EnableEntitiesRejectedMessage (1 functions)
 */

undefined4 * __thiscall Detail_EnableEntitiesRejectedMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01791368;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = ISGWMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== Detail_EntityCreateMessage.c ========== */

/*
 * SGW.exe - Detail_EntityCreateMessage (1 functions)
 */

undefined4 * __thiscall Detail_EntityCreateMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017913b3;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_00dd3d10((undefined4 *)((int)this + 0x14));
  local_4 = 0xffffffff;
  *(undefined ***)this = ISGWMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== Detail_EntityLeaveMessage.c ========== */

/*
 * SGW.exe - Detail_EntityLeaveMessage (1 functions)
 */

undefined4 * __thiscall Detail_EntityLeaveMessage__vfunc_0(void *this,byte param_1)

{
  FUN_015620b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Detail_EntityPropertiesMessage.c ========== */

/*
 * SGW.exe - Detail_EntityPropertiesMessage (1 functions)
 */

undefined4 * __thiscall Detail_EntityPropertiesMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017913f3;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_00dd3d10((undefined4 *)((int)this + 8));
  local_4 = 0xffffffff;
  *(undefined ***)this = ISGWMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== Detail_SpaceDataMessage.c ========== */

/*
 * SGW.exe - Detail_SpaceDataMessage (1 functions)
 */

undefined4 * __thiscall Detail_SpaceDataMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01562210(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DirSection.c ========== */

/*
 * SGW.exe - DirSection (19 functions)
 */

/* [VTable] DirSection virtual function #12
   VTable: vtable_DirSection at 017fc2d4 */

undefined4 DirSection__vfunc_12(void)

{
  return 0x400;
}


/* [VTable] DirSection virtual function #1
   VTable: vtable_DirSection at 017fc2d4 */

int __fastcall DirSection__vfunc_1(int param_1)

{
  if (*(int *)(param_1 + 0x44) == 0) {
    return 0;
  }
  return (*(int *)(param_1 + 0x48) - *(int *)(param_1 + 0x44)) / 0x1c;
}


/* [VTable] DirSection virtual function #3
   VTable: vtable_DirSection at 017fc2d4 */

undefined4 __thiscall DirSection__vfunc_3(void *this,undefined4 param_1,uint param_2)

{
  void *unaff_ESI;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167bac9;
  pvStack_c = ExceptionList;
  if ((*(int *)((int)this + 0x44) == 0) ||
     (ExceptionList = &pvStack_c,
     (uint)((*(int *)((int)this + 0x48) - *(int *)((int)this + 0x44)) / 0x1c) <= param_2)) {
    ExceptionList = &pvStack_c;
    _invalid_parameter_noinfo();
  }
  (**(code **)(*(int *)this + 0x1c))(param_1,*(int *)((int)this + 0x44) + param_2 * 0x1c);
  ExceptionList = unaff_ESI;
  return param_1;
}


/* [VTable] DirSection virtual function #13
   VTable: vtable_DirSection at 017fc2d4 */

uint __thiscall DirSection__vfunc_13(void *this,void *param_1)

{
  char cVar1;
  uint uVar2;
  int iVar3;
  int *unaff_EBX;
  int iVar4;
  undefined4 uStack_30;
  char acStack_2c [12];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167bc90;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  uVar2 = std::char_traits<char>::length("");
  uVar2 = FUN_004242c0(param_1,0,*(uint *)((int)param_1 + 0x14),"",uVar2);
  if (uVar2 != 0) {
    ExceptionList = pvStack_c;
    return uVar2 & 0xffffff00;
  }
  iVar4 = 0;
  iVar3 = (**(code **)(*(int *)this + 4))();
  if (0 < iVar3) {
    do {
      (**(code **)(*(int *)this + 0xc))(acStack_2c,iVar4);
      pvStack_c = (void *)0x0;
      pvStack_18 = (void *)0xf;
      uStack_4 = uStack_4 & 0xffffff00;
      uStack_1c = 0;
      std::char_traits<char>::assign(acStack_2c,(char *)&uStack_4);
      pvStack_c = (void *)CONCAT31(pvStack_c._1_3_,1);
      cVar1 = (**(code **)(*unaff_EBX + 0x34))(&uStack_30);
      cVar1 = cVar1 == '\0';
      uStack_10 = uStack_10 & 0xffffff00;
      if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
        scalable_free(uStack_30);
      }
      uStack_1c = 0xf;
      puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
      uStack_20 = 0;
      std::char_traits<char>::assign((char *)&uStack_30,(char *)&puStack_8);
      if (cVar1 != '\0') {
        uStack_10 = 0xffffffff;
        uVar2 = FUN_004585a0((int *)&stack0xffffffc8);
        ExceptionList = pvStack_18;
        return uVar2 & 0xffffff00;
      }
      uStack_10 = 0xffffffff;
      FUN_004585a0((int *)&stack0xffffffc8);
      iVar4 = iVar4 + 1;
      iVar3 = (**(code **)(*(int *)this + 4))();
    } while (iVar4 < iVar3);
  }
  ExceptionList = pvStack_c;
  return CONCAT31((int3)((uint)iVar3 >> 8),1);
}


/* [VTable] DirSection virtual function #4
   VTable: vtable_DirSection at 017fc2d4 */

void * __thiscall DirSection__vfunc_4(void *this,void *param_1,uint param_2)

{
  int iVar1;
  uint uVar2;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uVar2 = param_2;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01762009;
  pvStack_c = ExceptionList;
  if ((*(int *)((int)this + 0x44) == 0) ||
     (ExceptionList = &pvStack_c,
     (uint)((*(int *)((int)this + 0x48) - *(int *)((int)this + 0x44)) / 0x1c) <= param_2)) {
    ExceptionList = &pvStack_c;
    _invalid_parameter_noinfo();
  }
  iVar1 = *(int *)((int)this + 0x44);
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  param_2 = param_2 & 0xffffff00;
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<char>::assign((char *)((int)param_1 + 4),(char *)&param_2);
  FUN_00437710(param_1,(void *)(iVar1 + uVar2 * 0x1c),0,0xffffffff);
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] DirSection virtual function #11
   VTable: vtable_DirSection at 017fc2d4 */

void * __thiscall DirSection__vfunc_11(void *this,void *param_1)

{
  char local_11 [5];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01762009;
  pvStack_c = ExceptionList;
  local_11[1] = '\0';
  local_11[2] = '\0';
  local_11[3] = '\0';
  local_11[4] = '\0';
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  local_11[0] = '\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<char>::assign((char *)((int)param_1 + 4),local_11);
  FUN_00437710(param_1,(void *)((int)this + 0x24),0,0xffffffff);
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] DirSection virtual function #14
   VTable: vtable_DirSection at 017fc2d4 */

undefined1 __thiscall DirSection__vfunc_14(void *this,int *param_1)

{
  uint uVar1;
  undefined1 uVar2;
  int iVar3;
  void *pvVar4;
  char *pcVar5;
  undefined4 unaff_EBP;
  int *unaff_retaddr;
  undefined4 uStack_84;
  undefined1 auStack_80 [4];
  undefined4 uStack_7c;
  uint local_78 [4];
  undefined4 local_68;
  undefined4 local_64;
  char acStack_60 [16];
  undefined4 uStack_50;
  uint uStack_4c;
  undefined1 auStack_48 [4];
  char acStack_44 [16];
  undefined4 uStack_34;
  uint uStack_30;
  char acStack_28 [8];
  void *pvStack_20;
  uint uStack_18;
  uint uStack_14;
  int iStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167bd10;
  pvStack_c = ExceptionList;
  local_4 = 0;
  local_64 = 0xf;
  uStack_84._0_3_ = (uint3)(ushort)uStack_84;
  local_68 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_78,(char *)((int)&uStack_84 + 2));
  local_4 = CONCAT31(local_4._1_3_,1);
  if (*(int *)((int)this + 0x1c) != 0) {
    uVar1 = *(int *)((int)this + 0x1c) - 1;
    if (*(uint *)((int)this + 0x1c) < uVar1) {
      _invalid_parameter_noinfo();
    }
    if (*(uint *)((int)this + 0x20) < 0x10) {
      iVar3 = (int)this + 0xc;
    }
    else {
      iVar3 = *(int *)((int)this + 0xc);
    }
    if (*(char *)(iVar3 + uVar1) != '/') {
      iVar3 = (**(code **)(*param_1 + 0x2c))();
      puStack_8._0_1_ = 4;
      pvVar4 = BWResource__unknown_00459470(&local_64,(void *)((int)this + 8),"/");
      puStack_8._0_1_ = 5;
      pvVar4 = WinFileSystem__unknown_00458a50(auStack_48,pvVar4,iVar3);
      puStack_8._0_1_ = 6;
      FUN_00437710(auStack_80,pvVar4,0,0xffffffff);
      puStack_8._0_1_ = 5;
      if (0xf < uStack_30) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_30 = 0xf;
      uStack_34 = 0;
      std::char_traits<char>::assign(acStack_44,&stack0xffffff7a);
      puStack_8._0_1_ = 4;
      if (0xf < uStack_4c) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_4c = 0xf;
      uStack_50 = 0;
      std::char_traits<char>::assign(acStack_60,&stack0xffffff7a);
      puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,1);
      if (0xf < uStack_14) {
                    /* WARNING: Subroutine does not return */
        scalable_free();
      }
      uStack_14 = 0xf;
      uStack_18 = 0;
      pcVar5 = acStack_28;
      goto LAB_00460bfc;
    }
  }
  iVar3 = (**(code **)(*param_1 + 0x2c))();
  puStack_8._0_1_ = 2;
  pvVar4 = WinFileSystem__unknown_00458a50(&local_64,(void *)((int)this + 8),iVar3);
  puStack_8._0_1_ = 3;
  FUN_00437710(auStack_80,pvVar4,0,0xffffffff);
  puStack_8._0_1_ = 2;
  if (0xf < uStack_4c) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_4c = 0xf;
  uStack_50 = 0;
  std::char_traits<char>::assign(acStack_60,&stack0xffffff7a);
  puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,1);
  if (0xf < uStack_30) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_30 = 0xf;
  uStack_34 = 0;
  pcVar5 = acStack_44;
LAB_00460bfc:
  std::char_traits<char>::assign(pcVar5,&stack0xffffff7a);
  uStack_84 = &stack0xffffff60;
  (**(code **)(*unaff_retaddr + 0x70))(&stack0xffffff60);
  pvStack_c = (void *)CONCAT31((int3)((uint)pvStack_c >> 8),1);
  uVar2 = (**(code **)(**(int **)((int)this + 0x50) + 0x18))(&uStack_84);
  uStack_18 = uStack_18 & 0xffffff00;
  if (local_78[0] < 0x10) {
    local_78[0] = 0xf;
    uStack_7c = 0;
    std::char_traits<char>::assign(&stack0xffffff74,&stack0xffffff6a);
    uStack_18 = 0xffffffff;
    FUN_004585a0(&iStack_10);
    ExceptionList = pvStack_20;
    return uVar2;
  }
                    /* WARNING: Subroutine does not return */
  scalable_free(unaff_EBP);
}


/* Also in vtable: DirSection (slot 0) */

undefined4 * __thiscall DirSection__vfunc_0(void *this,byte param_1)

{
  FUN_00460df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] DirSection virtual function #7
   VTable: vtable_DirSection at 017fc2d4 */

void * __thiscall DirSection__vfunc_7(void *this,void *param_1,void *param_2)

{
  uint uVar1;
  int *piVar2;
  uint3 uVar3;
  void *pvVar4;
  int *piVar5;
  int iVar6;
  undefined4 *puVar7;
  undefined *this_00;
  int *extraout_ECX;
  int extraout_ECX_00;
  int extraout_ECX_01;
  int extraout_ECX_02;
  int extraout_ECX_03;
  int iVar8;
  int *extraout_ECX_04;
  char *pcVar9;
  undefined4 **ppuVar10;
  undefined1 *puVar11;
  char cVar12;
  char local_79;
  int *piStack_78;
  int aiStack_74 [2];
  undefined4 *puStack_6c;
  undefined1 *puStack_68;
  undefined1 *puStack_64;
  undefined1 auStack_60 [4];
  char local_5c [16];
  undefined4 local_4c;
  uint local_48;
  undefined1 auStack_44 [4];
  char acStack_40 [16];
  undefined4 uStack_30;
  uint uStack_2c;
  undefined1 auStack_28 [4];
  char acStack_24 [16];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167be64;
  pvStack_c = ExceptionList;
  aiStack_74[1] = 0;
  local_48 = 0xf;
  local_79 = '\0';
  local_4c = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_5c,&local_79);
  piStack_78 = (int *)0x0;
  iStack_4._1_3_ = 0;
  uVar3 = iStack_4._1_3_;
  iStack_4._0_1_ = 3;
  iStack_4._1_3_ = 0;
  if (*(int *)((int)this + 0x1c) == 0) {
LAB_0046156d:
    iStack_4._1_3_ = uVar3;
    pvVar4 = WinFileSystem__unknown_00458a50(auStack_44,(void *)((int)this + 8),(int)param_2);
    iStack_4._0_1_ = 4;
    FUN_00437710(auStack_60,pvVar4,0,0xffffffff);
    iStack_4._0_1_ = 3;
    if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free();
    }
    pcVar9 = acStack_40;
    uStack_2c = 0xf;
    uStack_30 = 0;
  }
  else {
    uVar1 = *(int *)((int)this + 0x1c) - 1;
    if (*(uint *)((int)this + 0x1c) < uVar1) {
      _invalid_parameter_noinfo();
    }
    if (*(uint *)((int)this + 0x20) < 0x10) {
      iVar6 = (int)this + 0xc;
    }
    else {
      iVar6 = *(int *)((int)this + 0xc);
    }
    uVar3 = iStack_4._1_3_;
    if (*(char *)(iVar6 + uVar1) == '/') goto LAB_0046156d;
    pvVar4 = BWResource__unknown_00459470(auStack_28,(void *)((int)this + 8),"/");
    iStack_4._0_1_ = 5;
    pvVar4 = WinFileSystem__unknown_00458a50(auStack_44,pvVar4,(int)param_2);
    iStack_4._0_1_ = 6;
    FUN_00437710(auStack_60,pvVar4,0,0xffffffff);
    iStack_4._0_1_ = 5;
    if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free();
    }
    uStack_2c = 0xf;
    local_79 = '\0';
    uStack_30 = 0;
    std::char_traits<char>::assign(acStack_40,&local_79);
    iStack_4._0_1_ = 3;
    if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free();
    }
    pcVar9 = acStack_24;
    uStack_10 = 0xf;
    uStack_14 = 0;
  }
  iStack_4._0_1_ = 3;
  local_79 = '\0';
  std::char_traits<char>::assign(pcVar9,&local_79);
  puVar11 = auStack_60;
  ppuVar10 = &puStack_6c;
  pvVar4 = (void *)DirSection__unknown_004603e0();
  piVar5 = FUN_0045f920(pvVar4,ppuVar10,puVar11);
  piVar2 = piStack_78;
  iStack_4._0_1_ = 7;
  piVar5 = (int *)*piVar5;
  if (piStack_78 != piVar5) {
    piStack_78 = piVar5;
    if (piVar5 != (int *)0x0) {
      FUN_00457e40((int)piVar5);
    }
    if ((piVar2 != (int *)0x0) && (iVar6 = FUN_00457e50((int)piVar2), iVar6 == 1)) {
      (**(code **)*piVar2)();
    }
  }
  iStack_4._0_1_ = 3;
  FUN_004585a0((int *)&puStack_6c);
  if (piStack_78 != (int *)0x0) {
    FUN_00458600(param_1,(int *)&piStack_78);
    aiStack_74[1] = 1;
    iStack_4._0_1_ = 1;
    FUN_004585a0((int *)&piStack_78);
    iStack_4 = (uint)iStack_4._1_3_ << 8;
    if (local_48 < 0x10) {
      local_48 = 0xf;
      local_79 = '\0';
      local_4c = 0;
      std::char_traits<char>::assign(local_5c,&local_79);
      ExceptionList = pvStack_c;
      return param_1;
    }
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  puVar7 = FUN_0045f620(&puStack_6c,auStack_60);
  piVar2 = piStack_78;
  iStack_4._0_1_ = 8;
  piVar5 = (int *)*puVar7;
  if (piStack_78 != piVar5) {
    piStack_78 = piVar5;
    if (piVar5 != (int *)0x0) {
      FUN_00457e40((int)piVar5);
    }
    if ((piVar2 != (int *)0x0) && (iVar6 = FUN_00457e50((int)piVar2), iVar6 == 1)) {
      (**(code **)*piVar2)();
    }
  }
  iStack_4._0_1_ = 3;
  FUN_004585a0((int *)&puStack_6c);
  if (piStack_78 == (int *)0x0) {
    iVar6 = (**(code **)(**(int **)((int)this + 0x50) + 4))();
    if (iVar6 == 1) {
      puStack_64 = (undefined1 *)FUN_00418e30(0x54);
      iStack_4._0_1_ = 10;
      if (puStack_64 == (undefined1 *)0x0) {
        puStack_6c = (undefined4 *)0x0;
      }
      else {
        puStack_6c = FUN_00461150(puStack_64,auStack_60,*(undefined4 *)((int)this + 0x50));
      }
      iStack_4 = CONCAT31(iStack_4._1_3_,3);
      FUN_00437710(puStack_6c + 9,param_2,0,0xffffffff);
      if (puStack_6c != (undefined4 *)0x0) {
        FUN_00457e40((int)puStack_6c);
      }
      iStack_4._0_1_ = 0xc;
      FUN_0046bd00(&piStack_78,(int *)&puStack_6c);
      iStack_4._0_1_ = 3;
      FUN_004585a0((int *)&puStack_6c);
      iVar8 = extraout_ECX_03;
    }
    else {
      iVar8 = extraout_ECX_00;
      if (iVar6 == 2) {
        (**(code **)(**(int **)((int)this + 0x50) + 0x10))();
        iStack_4._0_1_ = 0xd;
        puVar11 = auStack_60;
        this_00 = FUN_00470de0();
        FUN_00470e30(this_00,(int)puVar11);
        if (aiStack_74[0] != 0) {
          cVar12 = '\x01';
          puStack_64 = &stack0xffffff6c;
          iVar6 = extraout_ECX_01;
          FUN_00458600(&stack0xffffff6c,aiStack_74);
          iStack_4._0_1_ = 0xd;
          piVar5 = FUN_00467360(&puStack_68,param_2,iVar6,cVar12);
          iStack_4._0_1_ = 0xf;
          FUN_0046bd00(&piStack_78,piVar5);
          iStack_4._0_1_ = 0xd;
          FUN_004585a0((int *)&puStack_68);
          piVar5 = piStack_78;
          if (piStack_78 != (int *)0x0) {
            puStack_64 = &stack0xffffff70;
            FUN_0046bcb0(&stack0xffffff70,(int)this,'\0');
            iStack_4._0_1_ = 0xd;
            (**(code **)(*piVar5 + 0x3c))();
          }
        }
        iStack_4._0_1_ = 3;
        FUN_004585a0(aiStack_74);
        iVar8 = extraout_ECX_02;
      }
    }
    if (piStack_78 == (int *)0x0) goto LAB_00461865;
    puStack_64 = &stack0xffffff70;
    FUN_00458600(&stack0xffffff70,(int *)&piStack_78);
    iStack_4._0_1_ = 3;
    piVar5 = FUN_0045f680((int *)&puStack_68,auStack_60,iVar8);
    iStack_4._0_1_ = 0x12;
    FUN_0046bd00(&piStack_78,piVar5);
    iStack_4._0_1_ = 3;
    FUN_004585a0((int *)&puStack_68);
    puStack_64 = &stack0xffffff70;
    piVar5 = extraout_ECX_04;
    FUN_00458600(&stack0xffffff70,(int *)&piStack_78);
    iStack_4._0_1_ = 0x13;
  }
  else {
    puStack_68 = &stack0xffffff70;
    piVar5 = extraout_ECX;
    FUN_00458600(&stack0xffffff70,(int *)&piStack_78);
    iStack_4._0_1_ = 9;
  }
  puVar11 = auStack_60;
  pvVar4 = (void *)DirSection__unknown_004603e0();
  iStack_4._0_1_ = 3;
  DirSection__unknown_004600c0(pvVar4,puVar11,piVar5);
LAB_00461865:
  FUN_00458600(param_1,(int *)&piStack_78);
  aiStack_74[1] = 1;
  iStack_4._0_1_ = 1;
  FUN_004585a0((int *)&piStack_78);
  iStack_4 = (uint)iStack_4._1_3_ << 8;
  ZipFileSystem__unknown_00433ed0((uint)auStack_60);
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] DirSection virtual function #5
   VTable: vtable_DirSection at 017fc2d4 */

undefined4 __thiscall DirSection__vfunc_5(void *this,undefined4 param_1,void *param_2)

{
  uint uVar1;
  undefined1 *puVar2;
  char cVar3;
  int iVar4;
  void *pvVar5;
  int *piVar6;
  undefined4 unaff_retaddr;
  undefined1 *puVar7;
  undefined4 uStack_84;
  char acStack_80 [4];
  undefined4 local_7c;
  undefined1 *local_78;
  undefined1 *puStack_74;
  undefined1 *apuStack_70 [2];
  undefined4 uStack_68;
  undefined1 auStack_64 [4];
  undefined1 auStack_60 [4];
  char local_5c [4];
  undefined4 uStack_58;
  uint uStack_54;
  undefined4 local_4c;
  undefined4 local_48;
  undefined1 auStack_44 [4];
  char acStack_40 [16];
  undefined4 uStack_30;
  uint uStack_2c;
  undefined1 auStack_28 [16];
  void *pvStack_18;
  undefined1 uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167beff;
  pvStack_c = ExceptionList;
  local_7c = 0;
  local_78 = (undefined1 *)0x0;
  local_4 = 2;
  local_48 = 0xf;
  uStack_84._3_1_ = 0;
  local_4c = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_5c,(char *)((int)&uStack_84 + 3));
  local_4._0_1_ = 3;
  if (*(int *)((int)this + 0x1c) != 0) {
    uVar1 = *(int *)((int)this + 0x1c) - 1;
    if (*(uint *)((int)this + 0x1c) < uVar1) {
      _invalid_parameter_noinfo();
    }
    if (*(uint *)((int)this + 0x20) < 0x10) {
      iVar4 = (int)this + 0xc;
    }
    else {
      iVar4 = *(int *)((int)this + 0xc);
    }
    if (*(char *)(iVar4 + uVar1) != '/') {
      pvVar5 = BWResource__unknown_00459470(auStack_28,(void *)((int)this + 8),"/");
      local_4._0_1_ = 5;
      pvVar5 = WinFileSystem__unknown_00458a50(auStack_44,pvVar5,(int)param_2);
      local_4._0_1_ = 6;
      FUN_00437710(auStack_60,pvVar5,0,0xffffffff);
      local_4._0_1_ = 5;
      ZipFileSystem__unknown_00433ed0((uint)auStack_44);
      local_4 = CONCAT31(local_4._1_3_,3);
      ZipFileSystem__unknown_00433ed0((uint)auStack_28);
      goto LAB_00461a26;
    }
  }
  pvVar5 = WinFileSystem__unknown_00458a50(auStack_44,(void *)((int)this + 8),(int)param_2);
  local_4._0_1_ = 4;
  FUN_00437710(auStack_60,pvVar5,0,0xffffffff);
  local_4 = CONCAT31(local_4._1_3_,3);
  if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_2c = 0xf;
  acStack_80[0] = '\0';
  uStack_30 = 0;
  std::char_traits<char>::assign(acStack_40,acStack_80);
LAB_00461a26:
  acStack_80[0] = '.';
  piVar6 = FUN_00457ef0(param_2,acStack_80,(int *)0x0,1);
  if (((int)piVar6 < 0) || (*(int *)((int)param_2 + 0x14) <= (int)piVar6)) {
    cVar3 = (**(code **)(**(int **)((int)this + 0x50) + 0x14))();
    if (cVar3 != '\0') {
      puStack_74 = (undefined1 *)scalable_malloc();
      if (puStack_74 == (void *)0x0) {
        FUN_004099f0((exception *)apuStack_70);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(apuStack_70,(ThrowInfo *)&DAT_01d65cc8);
      }
      puStack_8._0_1_ = 0xc;
      piVar6 = FUN_00461150(puStack_74,auStack_64,*(undefined4 *)((int)this + 0x50));
      puStack_8._0_1_ = 3;
      puStack_74 = &stack0xffffff64;
      local_78 = &stack0xffffff64;
      puVar7 = &stack0xffffff64;
      puVar2 = &stack0xffffff64;
      if (piVar6 != (int *)0x0) {
        FUN_00457e40((int)piVar6);
        puVar7 = local_78;
        puVar2 = puStack_74;
      }
      puStack_74 = puVar2;
      local_78 = puVar7;
      puStack_8._0_1_ = 0xe;
      puVar7 = auStack_64;
      pvVar5 = (void *)DirSection__unknown_004603e0();
      puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,3);
      DirSection__unknown_004600c0(pvVar5,puVar7,piVar6);
      FUN_0045b4b0((void *)((int)this + 0x40),param_2);
    }
  }
  else {
    puStack_74 = (undefined1 *)FUN_0046c4b0();
    local_4._0_1_ = 7;
    if (puStack_74 == (void *)0x0) {
      piVar6 = (int *)0x0;
    }
    else {
      piVar6 = FUN_0046f640(puStack_74,param_2);
    }
    local_4._0_1_ = 3;
    puStack_74 = &stack0xffffff68;
    apuStack_70[0] = &stack0xffffff68;
    FUN_00457e40((int)this);
    local_4 = CONCAT31(local_4._1_3_,3);
    (**(code **)(*piVar6 + 0x3c))();
    puStack_74 = &stack0xffffff64;
    local_78 = &stack0xffffff64;
    FUN_00457e40((int)piVar6);
    puStack_8._0_1_ = 0xb;
    puVar7 = auStack_64;
    pvVar5 = (void *)DirSection__unknown_004603e0();
    puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,3);
    DirSection__unknown_004600c0(pvVar5,puVar7,piVar6);
  }
  (**(code **)(*(int *)this + 0x1c))(unaff_retaddr);
  uStack_10 = 2;
  if (0xf < uStack_54) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_68);
  }
  uStack_54 = 0xf;
  uStack_58 = 0;
  std::char_traits<char>::assign((char *)&uStack_68,&stack0xffffff74);
  uStack_10 = 0;
  FUN_004585a0(&uStack_84);
  ExceptionList = pvStack_18;
  return unaff_retaddr;
}


/* [VTable] DirSection virtual function #50
   VTable: vtable_DirSection at 017fc2d4 */

undefined4 * __fastcall DirSection__vfunc_50(undefined4 param_1,undefined *param_2)

{
  void *this;
  undefined4 *puVar1;
  void *unaff_retaddr;
  undefined4 uStack00000008;
  undefined4 in_stack_00000010;
  
  this = (void *)(*(code *)param_2)(0x178,8);
  uStack00000008 = 0;
  if (this == (void *)0x0) {
    puVar1 = (undefined4 *)0x0;
  }
  else {
    puVar1 = FUN_004b4130(this,0,0x370,0,0,L"UIImage",in_stack_00000010,L"Engine",0x4000,0x4084084,
                          FUN_00658d70,&DAT_0049eb80,&DAT_0049eb90);
  }
  uStack00000008 = 0xffffffff;
  if (puVar1 == (undefined4 *)0x0) {
    FUN_00486000("ReturnClass",".\\Src\\UnUIObjects.cpp",0x1e);
  }
  ExceptionList = unaff_retaddr;
  return puVar1;
}


/* [VTable] DirSection virtual function #57
   VTable: vtable_DirSection at 017fc2d4 */

void __thiscall DirSection__vfunc_57(void *this)

{
  bool bVar1;
  undefined **in_EAX;
  wchar_t *pwVar2;
  int *this_00;
  int iVar3;
  void *this_01;
  undefined **ppuVar4;
  int unaff_EBP;
  int *unaff_ESI;
  int unaff_EDI;
  undefined4 uStack00000004;
  uint in_stack_00000010;
  int in_stack_00000014;
  undefined **in_stack_00000018;
  int in_stack_0000001c;
  void *in_stack_0000003c;
  undefined4 in_stack_00000044;
  void *in_stack_0000004c;
  int in_stack_00000050;
  void *in_stack_00000054;
  int in_stack_00000058;
  
  ppuVar4 = this;
  do {
    pwVar2 = wcsstr((wchar_t *)in_EAX,(wchar_t *)ppuVar4);
    if (pwVar2 == (wchar_t *)0x0) {
LAB_006c7571:
      if (in_stack_00000050 != 0) {
        this_00 = FUN_0048eeb0((void *)(*(int *)(*(int *)(unaff_EBP + 200) + unaff_EDI) + 0x70),
                               (int *)&stack0x00000024);
        in_stack_00000044 = 5;
        in_stack_00000010 = in_stack_00000010 | 2;
        ppuVar4 = in_stack_00000018;
        if (in_stack_0000001c == 0) {
          ppuVar4 = &PTR_017f8e10;
        }
        iVar3 = FUN_0048f320(this_00,(wchar_t *)ppuVar4,0,0);
        if (iVar3 != -1) goto LAB_006c75c2;
      }
      bVar1 = false;
    }
    else {
      if (unaff_ESI[1] == 0) {
        ppuVar4 = &PTR_017f8e10;
      }
      else {
        ppuVar4 = (undefined **)*unaff_ESI;
      }
      if ((int)pwVar2 - (int)ppuVar4 >> 1 == -1) goto LAB_006c7571;
LAB_006c75c2:
      bVar1 = true;
    }
    in_stack_00000044 = 1;
    if ((in_stack_00000010 & 2) != 0) {
      in_stack_00000010 = in_stack_00000010 & 0xfffffffd;
      FUN_0041b420((int *)&stack0x00000024);
    }
    in_stack_00000044 = 0;
    if ((in_stack_00000010 & 1) != 0) {
      in_stack_00000010 = in_stack_00000010 & 0xfffffffe;
      FUN_0041b420((int *)&stack0x00000030);
    }
    if (bVar1) {
      FUN_005602a0(in_stack_00000054,(int *)(*(int *)(unaff_EBP + 200) + unaff_EDI));
    }
    if (in_stack_00000058 != 0) {
      this_01 = (void *)FUN_006d7f20(*(int *)(*(int *)(unaff_EBP + 200) + unaff_EDI));
      if (this_01 != (void *)0x0) {
        FUN_006c74b0(this_01,in_stack_0000004c,in_stack_00000050,in_stack_00000054,1);
      }
    }
    in_stack_00000014 = in_stack_00000014 + 1;
    if (*(int *)(unaff_EBP + 0xcc) <= in_stack_00000014) {
      in_stack_00000044 = 0xffffffff;
      uStack00000004 = 0x6c7675;
      FUN_0041b420((int *)&stack0x00000018);
      ExceptionList = in_stack_0000003c;
      return;
    }
    unaff_EDI = in_stack_00000014 * 4;
    unaff_ESI = FUN_0048eeb0((void *)(*(int *)(*(int *)(unaff_EBP + 200) + unaff_EDI) + 0x50),
                             (int *)&stack0x00000030);
    in_stack_00000044 = CONCAT31(in_stack_00000044._1_3_,1);
    in_stack_00000010 = in_stack_00000010 | 1;
    ppuVar4 = in_stack_00000018;
    if (in_stack_0000001c == 0) {
      ppuVar4 = &PTR_017f8e10;
    }
    if (unaff_ESI[1] != 0) {
      DirSection__vfunc_57(ppuVar4);
      return;
    }
    in_EAX = &PTR_017f8e10;
  } while( true );
}


/* [VTable] DirSection virtual function #44
   VTable: vtable_DirSection at 017fc2d4 */

void DirSection__vfunc_44(void)

{
  int *unaff_EBX;
  int unaff_EDI;
  
  *unaff_EBX = *unaff_EBX + unaff_EDI;
  return;
}


/* [VTable] DirSection virtual function #43
   VTable: vtable_DirSection at 017fc2d4 */

void DirSection__vfunc_43(void)

{
  byte bVar1;
  char cVar2;
  undefined4 *puVar3;
  undefined4 *puVar4;
  char cVar5;
  undefined4 *puVar6;
  undefined4 uVar7;
  undefined4 *unaff_EBX;
  int unaff_ESI;
  undefined4 unaff_EDI;
  bool in_ZF;
  undefined4 uStack00000008;
  undefined4 *in_stack_00000010;
  undefined4 *in_stack_00000014;
  int *in_stack_00000018;
  undefined4 in_stack_00000020;
  int in_stack_00000024;
  undefined8 *in_stack_0000002c;
  undefined8 *in_stack_00000030;
  uint uStack00000034;
  undefined4 *in_stack_00000038;
  ushort *puStack0000003c;
  
  if (in_ZF) {
    if (DAT_01ee3998 == unaff_EBX) {
      DAT_01ee3998 = FUN_0076f700();
      FUN_00769330();
    }
    in_stack_00000010 = DAT_01ee3998;
  }
  puVar6 = (undefined4 *)
           FUN_008721b0(DAT_01ee2684,&DAT_01ea5750,(float *)&stack0x0000008c,
                        (float *)&stack0x0000004c,(float *)&stack0x00000040,0x209e,unaff_EDI,
                        unaff_EBX);
  uStack00000034 = (uint)**(ushort **)(unaff_ESI + 0xc);
  puStack0000003c = *(ushort **)(unaff_ESI + 0xc) + 1;
  *(ushort **)(unaff_ESI + 0xc) = puStack0000003c;
  cVar5 = '\0';
  do {
    if (puVar6 == unaff_EBX) {
      *(uint *)(unaff_ESI + 0xc) = *(int *)(*(int *)(unaff_ESI + 4) + 0x54) + 1 + uStack00000034;
      *in_stack_00000018 = (int)unaff_EBX;
      break;
    }
    puVar3 = (undefined4 *)puVar6[1];
    if ((puVar3 == unaff_EBX) || ((*(byte *)(puVar3 + 0x1c) & 8) != 0)) {
LAB_006e76ce:
      puVar6 = (undefined4 *)*puVar6;
    }
    else {
      for (puVar4 = (undefined4 *)puVar3[0xd]; puVar4 != unaff_EBX;
          puVar4 = (undefined4 *)puVar4[0xf]) {
        if (puVar4 == in_stack_00000010) goto LAB_006e7601;
      }
      if ((undefined4 *)(uint)(in_stack_00000010 == unaff_EBX) == unaff_EBX) goto LAB_006e76ce;
LAB_006e7601:
      *in_stack_00000018 = (int)puVar3;
      *in_stack_00000030 = *(undefined8 *)(puVar6 + 2);
      *(undefined4 *)(in_stack_00000030 + 1) = puVar6[4];
      *in_stack_0000002c = *(undefined8 *)(puVar6 + 5);
      *(undefined4 *)(in_stack_0000002c + 1) = puVar6[7];
      if (in_stack_00000038 != unaff_EBX) {
        FUN_006e8830((int)puVar6,(int)in_stack_00000014);
        if ((undefined4 *)puVar6[10] == unaff_EBX) {
          uVar7 = 0;
        }
        else {
          uVar7 = (**(code **)(*(int *)puVar6[10] + 0x124))();
        }
        *in_stack_00000014 = uVar7;
        in_stack_00000014[2] = puVar6[9];
        in_stack_00000014[3] = puVar6[0x10];
        in_stack_00000014[4] = puVar6[0xd];
        in_stack_00000014[5] = puVar6[0xe];
        in_stack_00000014[6] = puVar6[0xc];
      }
      puVar6 = (undefined4 *)*puVar6;
      cVar5 = **(char **)(unaff_ESI + 0xc);
      while ((cVar5 != '0' && (cVar5 != '1'))) {
        bVar1 = **(byte **)(unaff_ESI + 0xc);
        *(byte **)(unaff_ESI + 0xc) = *(byte **)(unaff_ESI + 0xc) + 1;
        (*(code *)(&DAT_01edcbd0)[bVar1])();
        cVar5 = **(char **)(unaff_ESI + 0xc);
      }
      cVar2 = **(char **)(unaff_ESI + 0xc);
      *(char **)(unaff_ESI + 0xc) = *(char **)(unaff_ESI + 0xc) + 1;
      if (cVar2 == '1') {
        *(ushort **)(unaff_ESI + 0xc) = puStack0000003c;
      }
    }
  } while (cVar5 != '0');
  if (in_stack_00000024 != DAT_01ea575c) {
    uStack00000008 = 0x6e770c;
    FUN_004fc360(&DAT_01ea5750,in_stack_00000024);
    DAT_01ea5750 = in_stack_00000020;
    return;
  }
  DAT_01ea5750 = in_stack_00000020;
  return;
}


/* [VTable] DirSection virtual function #59
   VTable: vtable_DirSection at 017fc2d4 */

void DirSection__vfunc_59(void)

{
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] DirSection virtual function #53
   VTable: vtable_DirSection at 017fc2d4 */

void DirSection__vfunc_53(void)

{
  int *piVar1;
  undefined8 uVar2;
  code *pcVar3;
  bool bVar4;
  int iVar5;
  int unaff_EBP;
  int *unaff_ESI;
  undefined1 in_CF;
  undefined1 in_ZF;
  float10 fVar6;
  float fVar7;
  float fVar8;
  float fVar9;
  float fVar10;
  float fVar11;
  float in_XMM1_Da;
  undefined1 auVar12 [16];
  float fStack00000014;
  float fStack00000018;
  float in_stack_0000001c;
  float in_stack_00000020;
  float in_stack_00000024;
  float in_stack_00000028;
  float fStack00000030;
  float fStack00000034;
  float in_stack_00000038;
  float in_stack_0000003c;
  float in_stack_00000040;
  float in_stack_00000044;
  float in_stack_00000048;
  float in_stack_0000004c;
  float in_stack_00000050;
  float in_stack_00000054;
  float in_stack_00000058;
  float fStack0000005c;
  float fStack00000060;
  float in_stack_00000064;
  int in_stack_00000068;
  float in_stack_0000006c;
  undefined8 in_stack_00000070;
  float in_stack_00000078;
  float fStack0000007c;
  float fStack00000080;
  int in_stack_00000084;
  float in_stack_00000088;
  undefined8 in_stack_0000008c;
  float in_stack_00000094;
  undefined4 in_stack_0000009c;
  float in_stack_000000ac;
  float in_stack_000000b0;
  float in_stack_000000b4;
  float in_stack_000000b8;
  undefined4 in_stack_000000c8;
  float in_stack_000000e4;
  float in_stack_000000e8;
  float in_stack_000000ec;
  float in_stack_000000f0;
  float in_stack_000000f4;
  float in_stack_000000f8;
  float in_stack_000000fc;
  float in_stack_00000100;
  float in_stack_00000104;
  float in_stack_00000108;
  float in_stack_0000010c;
  float in_stack_00000110;
  float in_stack_00000114;
  float in_stack_00000118;
  float in_stack_0000011c;
  float in_stack_00000120;
  float in_stack_00000124;
  float in_stack_00000128;
  float in_stack_0000012c;
  float in_stack_00000130;
  undefined8 in_stack_00000134;
  void *in_stack_0000013c;
  float in_stack_00000140;
  void *in_stack_00000154;
  undefined4 in_stack_0000015c;
  
  do {
    fVar9 = _DAT_0181a1f0;
    if (((bool)in_CF || (bool)in_ZF) || (7 < *(int *)(unaff_EBP + 0xc))) {
      if ((int *)unaff_ESI[0x74] != (int *)0x0) {
        (**(code **)(*(int *)unaff_ESI[0x74] + 0x36c))(*(undefined4 *)(unaff_EBP + 8));
      }
      *(ulonglong *)(unaff_ESI + 0x40) = CONCAT44(in_stack_000000f8,in_stack_000000f4);
      unaff_ESI[0x42] = (int)in_stack_000000fc;
      ExceptionList = in_stack_00000154;
      return;
    }
    *(int *)(unaff_EBP + 0xc) = *(int *)(unaff_EBP + 0xc) + 1;
    fVar11 = in_XMM1_Da;
    if ((fVar9 < in_XMM1_Da) &&
       (fVar11 = in_XMM1_Da * DAT_01816a8c, fVar9 <= in_XMM1_Da * DAT_01816a8c)) {
      fVar11 = fVar9;
    }
    in_stack_00000064 = (float)unaff_ESI[0x3f];
    fVar9 = (float)unaff_ESI[0x39];
    pcVar3 = *(code **)(*unaff_ESI + 0x128);
    unaff_ESI[0x1d] = unaff_ESI[0x1d] & 0xffbfffff;
    in_stack_00000048 = in_XMM1_Da - fVar11;
    _fStack00000030 = *(undefined8 *)(unaff_ESI + 0x37);
    _fStack0000005c = *(undefined8 *)(unaff_ESI + 0x3d);
    in_stack_00000038 = fVar9;
    fVar6 = (float10)(*pcVar3)();
    in_stack_0000006c = (float)fVar6;
    in_stack_00000108 = (float)unaff_ESI[0x40];
    in_stack_00000100 = (float)unaff_ESI[0x41];
    in_stack_0000010c = (float)unaff_ESI[0x42] + in_stack_0000006c;
    in_stack_000000e4 = 0.0;
    in_stack_0000004c = 0.0;
    (**(code **)(*unaff_ESI + 0x1f8))(&stack0x000000e4,&stack0x0000004c);
    fVar10 = DAT_017f7ea0;
    fVar7 = DAT_017f7ea0 - in_stack_000000e4;
    fVar8 = DAT_017f7ea0 - in_stack_0000004c * fVar11;
    in_stack_00000118 = fVar8 * in_stack_00000064 + fVar7 * in_stack_0000010c * fVar11;
    in_stack_00000110 = fStack0000005c * fVar8 + fVar7 * in_stack_00000108 * fVar11;
    in_stack_00000114 = fStack00000060 * fVar8 + fVar7 * in_stack_00000100 * fVar11;
    *(ulonglong *)(unaff_ESI + 0x3d) = CONCAT44(in_stack_00000114,in_stack_00000110);
    unaff_ESI[0x3f] = (int)in_stack_00000118;
    iVar5 = unaff_ESI[0x74];
    if (((iVar5 != 0) && ((*(byte *)(iVar5 + 0x1c0) & 0x40) != 0)) &&
       ((float)unaff_ESI[0x3f] <= 0.0)) {
      unaff_ESI[0x1d] = unaff_ESI[0x1d] | 0x400000;
      *(uint *)(iVar5 + 0x1c0) = *(uint *)(iVar5 + 0x1c0) & 0xffffffbf;
      (**(code **)(*(int *)unaff_ESI[0x74] + 0x37c))();
      fVar10 = DAT_017f7ea0;
    }
    uVar2 = _fStack0000007c;
    if ((in_stack_00000068 == 0) && (in_stack_00000088 != 0.0)) {
      uVar2 = *(undefined8 *)(unaff_ESI + 0x3d);
      fStack0000007c = (float)uVar2;
      fStack00000080 = (float)((ulonglong)uVar2 >> 0x20);
      fVar7 = fStack0000007c * fStack0000007c + fStack00000080 * fStack00000080;
      in_stack_00000084 = 0;
      if (in_stack_00000088 * in_stack_00000088 < fVar7) {
        if (fVar7 == fVar10) {
          in_stack_0000013c = (void *)0x0;
          in_stack_00000124 = 0.0;
          in_stack_00000134 = uVar2;
        }
        else if (DAT_01816ac0 <= fVar7) {
          if ((DAT_01ee0d70 & 1) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 1;
            _DAT_01ee0d60 = DAT_01816a80;
            uRam01ee0d64 = 0;
            uRam01ee0d68 = 0;
            uRam01ee0d6c = 0;
            in_stack_0000015c = 0xffffffff;
          }
          if ((DAT_01ee0d70 & 2) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 2;
            _DAT_01ee0d50 = DAT_01816a8c;
            uRam01ee0d54 = 0;
            uRam01ee0d58 = 0;
            uRam01ee0d5c = 0;
            in_stack_0000015c = 0xffffffff;
          }
          auVar12 = rsqrtss(ZEXT416((uint)fVar7),ZEXT416((uint)fVar7));
          fVar10 = auVar12._0_4_;
          in_stack_00000140 = _DAT_01ee0d50 * fVar10 * (_DAT_01ee0d60 - fVar7 * fVar10 * fVar10);
          fStack00000080 = fStack00000080 * in_stack_00000140;
          fStack0000007c = in_stack_00000140 * fStack0000007c;
          in_stack_00000124 = in_stack_00000140 * 0.0;
        }
        else {
          fStack0000007c = 0.0;
          fStack00000080 = 0.0;
          in_stack_00000124 = 0.0;
        }
        in_stack_00000124 = in_stack_00000124 * in_stack_00000088;
        in_stack_0000011c = fStack0000007c * in_stack_00000088;
        in_stack_00000084 = unaff_ESI[0x3f];
        in_stack_00000120 = fStack00000080 * in_stack_00000088;
        *(ulonglong *)(unaff_ESI + 0x3d) = CONCAT44(in_stack_00000120,in_stack_0000011c);
        unaff_ESI[0x3f] = in_stack_00000084;
      }
    }
    _fStack0000007c = uVar2;
    iVar5 = unaff_ESI[0x36];
    piVar1 = unaff_ESI + 0x3a;
    in_stack_0000003c = ((float)unaff_ESI[0x3d] + *(float *)(iVar5 + 0x1ec)) * fVar11;
    in_stack_00000040 = (*(float *)(iVar5 + 0x1f0) + (float)unaff_ESI[0x3e]) * fVar11;
    in_stack_00000044 = (*(float *)(iVar5 + 500) + (float)unaff_ESI[0x3f]) * fVar11;
    FUN_008730b0(DAT_01ee2684,unaff_ESI,&stack0x0000003c,piVar1,4,&stack0x00000098);
    if ((*(byte *)(unaff_ESI + 0x1c) & 8) != 0) {
      ExceptionList = in_stack_00000154;
      return;
    }
    if ((char)unaff_ESI[0x16] == '\x03') {
      FUN_007224c0(unaff_ESI,fStack00000030,fStack00000034,fVar9,(float)_fStack0000005c,
                   (float)((ulonglong)_fStack0000005c >> 0x20),in_stack_00000064,fVar11,
                   (DAT_017f7ea0 - in_stack_000000b8) * fVar11 + in_stack_00000048,
                   *(int *)(unaff_EBP + 0xc));
      ExceptionList = in_stack_00000154;
      return;
    }
    fVar9 = in_stack_00000064;
    if (in_stack_000000b8 < DAT_017f7ea0) {
      if ((float)unaff_ESI[0x73] <= in_stack_000000b4) {
        fVar9 = (DAT_017f7ea0 - in_stack_000000b8) * fVar11 + in_stack_00000048;
        in_stack_00000048 = fVar9;
        if ((((unaff_ESI[0x1d] & 0x400000U) == 0) && (DAT_01819a00 < in_stack_000000b8)) &&
           (_DAT_017f9000 < in_stack_000000b8 * fVar11)) {
          in_stack_00000028 = DAT_017f7ea0 / (in_stack_000000b8 * fVar11);
          in_stack_00000024 = ((float)unaff_ESI[0x38] - fStack00000034) * in_stack_00000028;
          in_stack_00000020 = in_stack_00000028 * ((float)unaff_ESI[0x37] - fStack00000030);
          in_stack_00000028 = ((float)unaff_ESI[0x39] - in_stack_00000038) * in_stack_00000028;
          *(ulonglong *)(unaff_ESI + 0x3d) = CONCAT44(in_stack_00000024,in_stack_00000020);
          unaff_ESI[0x3f] = (int)in_stack_00000028;
        }
LAB_007262ca:
        iVar5 = *unaff_ESI;
LAB_007262d5:
        (**(code **)(iVar5 + 0x1e4))
                  (in_stack_000000ac,in_stack_000000b0,in_stack_000000b4,in_stack_0000009c,fVar9,
                   *(undefined4 *)(unaff_EBP + 0xc));
        ExceptionList = in_stack_0000013c;
        return;
      }
      if ((((in_stack_000000f4 != 0.0) || (in_stack_000000f8 != 0.0)) || (in_stack_000000fc != 0.0))
         || (((int *)unaff_ESI[0x74] == (int *)0x0 ||
             (iVar5 = (**(code **)(*(int *)unaff_ESI[0x74] + 0x378))(fVar11,&stack0x000000f4),
             iVar5 == 0)))) {
        (**(code **)(*unaff_ESI + 0x1e0))
                  (in_stack_000000ac,in_stack_000000b0,in_stack_000000b4,in_stack_0000009c,
                   in_stack_000000c8);
        if ((*(byte *)(unaff_ESI + 0x1c) & 8) != 0) {
          ExceptionList = in_stack_00000154;
          return;
        }
        if ((char)unaff_ESI[0x16] != '\x02') {
          ExceptionList = in_stack_00000154;
          return;
        }
      }
      fStack00000014 = in_stack_000000ac;
      fStack00000018 = in_stack_000000b0;
      in_stack_0000001c = in_stack_000000b4;
      (**(code **)(*unaff_ESI + 0x398))(&stack0x00000050,&stack0x0000003c,&stack0x00000098);
      if (((float)PTR_017f94b8 <=
           in_stack_00000050 * in_stack_0000003c + in_stack_00000044 * in_stack_00000058 +
           in_stack_00000054 * in_stack_00000040) &&
         (FUN_008730b0(DAT_01ee2684,unaff_ESI,&stack0x00000050,piVar1,0,&stack0x00000098),
         in_stack_000000b8 < DAT_017f7ea0)) {
        iVar5 = *unaff_ESI;
        if ((float)unaff_ESI[0x73] <= in_stack_000000b4) {
          fVar9 = 0.0;
          goto LAB_007262d5;
        }
        (**(code **)(iVar5 + 0x1e0))
                  (in_stack_000000ac,in_stack_000000b0,in_stack_000000b4,in_stack_0000009c,
                   in_stack_000000c8);
        if ((*(byte *)(unaff_ESI + 0x1c) & 8) != 0) {
          ExceptionList = in_stack_00000154;
          return;
        }
        if ((char)unaff_ESI[0x16] != '\x02') {
          ExceptionList = in_stack_00000154;
          return;
        }
        fVar9 = in_stack_0000003c * in_stack_0000003c + in_stack_00000044 * in_stack_00000044 +
                in_stack_00000040 * in_stack_00000040;
        if (fVar9 == DAT_017f7ea0) {
          in_stack_0000008c = CONCAT44(in_stack_00000040,in_stack_0000003c);
          in_stack_00000094 = in_stack_00000044;
        }
        else {
          if (DAT_01816ac0 <= fVar9) {
            if ((DAT_01ee0d70 & 1) == 0) {
              DAT_01ee0d70 = DAT_01ee0d70 | 1;
              _DAT_01ee0d60 = DAT_01816a80;
              uRam01ee0d64 = 0;
              uRam01ee0d68 = 0;
              uRam01ee0d6c = 0;
              in_stack_0000015c = 0xffffffff;
            }
            if ((DAT_01ee0d70 & 2) == 0) {
              DAT_01ee0d70 = DAT_01ee0d70 | 2;
              _DAT_01ee0d50 = DAT_01816a8c;
              uRam01ee0d54 = 0;
              uRam01ee0d58 = 0;
              uRam01ee0d5c = 0;
              in_stack_0000015c = 0xffffffff;
            }
            auVar12 = rsqrtss(ZEXT416((uint)fVar9),ZEXT416((uint)fVar9));
            fVar10 = auVar12._0_4_;
            in_stack_00000140 = _DAT_01ee0d50 * fVar10 * (_DAT_01ee0d60 - fVar9 * fVar10 * fVar10);
            fVar9 = in_stack_00000140 * in_stack_0000003c;
            in_stack_0000008c._4_4_ = in_stack_00000040 * in_stack_00000140;
            in_stack_00000094 = in_stack_00000044 * in_stack_00000140;
          }
          else {
            fVar9 = 0.0;
            in_stack_0000008c._4_4_ = 0.0;
            in_stack_00000094 = 0.0;
          }
          in_stack_0000008c = CONCAT44(in_stack_0000008c._4_4_,fVar9);
        }
        FUN_007265b0((float *)&stack0x0000008c,&stack0x00000050,&stack0x000000ac,&stack0x00000014,
                     in_stack_000000b8);
        if ((((in_stack_0000001c <= 0.0) || (in_stack_000000b4 <= 0.0)) ||
            (in_stack_00000058 != 0.0)) ||
           (0.0 <= fStack00000014 * in_stack_000000ac + in_stack_0000001c * in_stack_000000b4 +
                   in_stack_000000b0 * fStack00000018)) {
          bVar4 = false;
        }
        else {
          bVar4 = true;
        }
        FUN_008730b0(DAT_01ee2684,unaff_ESI,&stack0x00000050,piVar1,0,&stack0x00000098);
        if ((bVar4) || ((float)unaff_ESI[0x73] <= in_stack_000000b4)) {
          fVar9 = 0.0;
          goto LAB_007262ca;
        }
      }
      in_stack_0000012c = DAT_017f7ea0 / fVar11;
      in_stack_00000128 = in_stack_0000012c * ((float)unaff_ESI[0x37] - fStack00000030);
      in_stack_00000130 = in_stack_0000012c * ((float)unaff_ESI[0x39] - in_stack_00000038);
      in_stack_0000012c = in_stack_0000012c * ((float)unaff_ESI[0x38] - fStack00000034);
      _fStack0000005c = CONCAT44(in_stack_0000012c,in_stack_00000128);
      fVar9 = in_stack_00000064;
      in_stack_00000064 = in_stack_00000130;
    }
    if (((in_stack_00000068 == 0) && ((unaff_ESI[0x1d] & 0x400000U) == 0)) &&
       ((char)unaff_ESI[0x16] != '\0')) {
      fVar11 = DAT_017f7ea0 / fVar11;
      in_stack_000000e8 = fVar11 * ((float)unaff_ESI[0x37] - fStack00000030);
      in_stack_000000f0 = fVar11 * ((float)unaff_ESI[0x39] - in_stack_00000038);
      in_stack_000000ec = fVar11 * ((float)unaff_ESI[0x38] - fStack00000034);
      *(ulonglong *)(unaff_ESI + 0x3d) = CONCAT44(in_stack_000000ec,in_stack_000000e8);
      unaff_ESI[0x3f] = (int)in_stack_000000f0;
      if (((float)unaff_ESI[0x3f] <= fVar9 && fVar9 != (float)unaff_ESI[0x3f]) ||
         ((float)PTR_017f94b8 <= fVar9)) {
        in_stack_00000020 = (float)unaff_ESI[0x3d] * DAT_01816a84 - fStack0000005c;
        in_stack_00000024 = (float)unaff_ESI[0x3e] * DAT_01816a84 - fStack00000060;
        in_stack_00000028 = (float)unaff_ESI[0x3f] * DAT_01816a84 - fVar9;
        *(ulonglong *)(unaff_ESI + 0x3d) = CONCAT44(in_stack_00000024,in_stack_00000020);
        unaff_ESI[0x3f] = (int)in_stack_00000028;
      }
      in_stack_00000104 =
           (float)unaff_ESI[0x3d] * (float)unaff_ESI[0x3d] +
           (float)unaff_ESI[0x3e] * (float)unaff_ESI[0x3e] +
           (float)unaff_ESI[0x3f] * (float)unaff_ESI[0x3f];
      fVar6 = (float10)(**(code **)(*unaff_ESI + 0x110))();
      in_stack_0000006c = (float)fVar6;
      if (in_stack_0000006c * in_stack_0000006c < in_stack_00000104) {
        fVar9 = (float)unaff_ESI[0x3d] * (float)unaff_ESI[0x3d] +
                (float)unaff_ESI[0x3e] * (float)unaff_ESI[0x3e] +
                (float)unaff_ESI[0x3f] * (float)unaff_ESI[0x3f];
        if (fVar9 == DAT_017f7ea0) {
          in_stack_00000078 = (float)unaff_ESI[0x3f];
          in_stack_00000070 = *(undefined8 *)(unaff_ESI + 0x3d);
        }
        else if (DAT_01816ac0 <= fVar9) {
          if ((DAT_01ee0d70 & 1) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 1;
            _DAT_01ee0d60 = DAT_01816a80;
            uRam01ee0d64 = 0;
            uRam01ee0d68 = 0;
            uRam01ee0d6c = 0;
            in_stack_0000015c = 0xffffffff;
          }
          if ((DAT_01ee0d70 & 2) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 2;
            _DAT_01ee0d50 = DAT_01816a8c;
            uRam01ee0d54 = 0;
            uRam01ee0d58 = 0;
            uRam01ee0d5c = 0;
            in_stack_0000015c = 0xffffffff;
          }
          auVar12 = rsqrtss(ZEXT416((uint)fVar9),ZEXT416((uint)fVar9));
          fVar11 = auVar12._0_4_;
          in_stack_00000140 = _DAT_01ee0d50 * fVar11 * (_DAT_01ee0d60 - fVar9 * fVar11 * fVar11);
          in_stack_00000078 = in_stack_00000140 * (float)unaff_ESI[0x3f];
          in_stack_00000070 =
               CONCAT44(in_stack_00000140 * (float)unaff_ESI[0x3e],
                        in_stack_00000140 * (float)unaff_ESI[0x3d]);
        }
        else {
          in_stack_00000078 = 0.0;
          in_stack_00000070 = 0;
        }
        *(undefined8 *)(unaff_ESI + 0x3d) = in_stack_00000070;
        unaff_ESI[0x3f] = (int)in_stack_00000078;
        fVar6 = (float10)(**(code **)(*unaff_ESI + 0x110))();
        unaff_ESI[0x3d] = (int)(float)(fVar6 * (float10)(float)unaff_ESI[0x3d]);
        unaff_ESI[0x3e] = (int)(float)(fVar6 * (float10)(float)unaff_ESI[0x3e]);
        unaff_ESI[0x3f] = (int)(float)(fVar6 * (float10)(float)unaff_ESI[0x3f]);
      }
    }
    in_ZF = in_stack_00000048 == (float)PTR_017f94b8;
    in_CF = in_stack_00000048 < (float)PTR_017f94b8;
    in_XMM1_Da = in_stack_00000048;
  } while( true );
}


/* [VTable] DirSection virtual function #54
   VTable: vtable_DirSection at 017fc2d4 */

void __thiscall DirSection__vfunc_54(void *this,undefined4 param_1,int *param_2)

{
  void *this_00;
  undefined4 uVar1;
  undefined4 uVar2;
  undefined4 uVar3;
  
  this_00 = (void *)((int)this + 0x58);
  if (*(int *)((int)this + 0x58) == 0) {
    FUN_006f0fc0();
    return;
  }
  if ((*(int *)((int)this + 0x74) == 0) && (*(int *)((int)this + 0x90) == 0)) {
    FUN_006f1010(this_00,param_2);
    return;
  }
  if (*(int *)((int)this + 0x74) == 0) {
    uVar1 = FUN_006f1010(this_00,param_2);
    uVar2 = FUN_006f1010((void *)((int)this + 0x90),param_2);
    (**(code **)(*param_2 + 0xb8))(uVar1,uVar2);
    return;
  }
  if (*(int *)((int)this + 0x90) == 0) {
    uVar1 = FUN_006f1010(this_00,param_2);
    uVar2 = FUN_006f1010((void *)((int)this + 0x74),param_2);
    (**(code **)(*param_2 + 0xbc))(uVar1,uVar2);
    return;
  }
  uVar1 = FUN_006f1010(this_00,param_2);
  uVar2 = FUN_006f1010((void *)((int)this + 0x74),param_2);
  uVar3 = FUN_006f1010((void *)((int)this + 0x90),param_2);
  (**(code **)(*param_2 + 0xc0))(uVar1,uVar2,uVar3);
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] DirSection virtual function #49
   VTable: vtable_DirSection at 017fc2d4 */

void __fastcall DirSection__vfunc_49(undefined8 *param_1,uint param_2)

{
  undefined4 *puVar1;
  int iVar2;
  int in_EAX;
  int iVar3;
  int iVar4;
  int unaff_EBX;
  int unaff_EBP;
  int unaff_ESI;
  int iVar5;
  int unaff_EDI;
  float10 extraout_ST0;
  float10 in_ST0;
  float10 fVar6;
  float in_XMM0_Da;
  float in_XMM1_Da;
  float in_XMM2_Da;
  float fVar7;
  float in_XMM3_Da;
  float fVar8;
  undefined1 auVar9 [16];
  float in_XMM4_Da;
  float in_XMM5_Da;
  float fVar10;
  float in_XMM7_Da;
  float fVar11;
  int iStack00000010;
  int in_stack_00000014;
  void *in_stack_00000018;
  float fStack0000001c;
  float fStack00000020;
  float fStack00000024;
  int iStack00000028;
  int in_stack_0000002c;
  float fStack00000030;
  int iStack00000034;
  undefined8 *puStack00000038;
  int in_stack_0000003c;
  uint uStack00000040;
  int iStack00000044;
  float fStack00000048;
  float fStack0000004c;
  float fStack00000050;
  float fStack00000054;
  float fStack00000068;
  float fStack0000006c;
  int in_stack_00000070;
  float fStack00000074;
  float in_stack_00000078;
  float in_stack_0000007c;
  int in_stack_00000080;
  float fStack00000084;
  float fStack00000088;
  float fStack0000008c;
  float fStack00000090;
  float fStack00000094;
  float fStack00000098;
  float fStack0000009c;
  float fStack000000a0;
  float fStack000000a4;
  float fStack000000a8;
  float fStack000000ac;
  float fStack000000b0;
  float fStack000000c8;
  float fStack000000cc;
  float fStack000000d0;
  float fStack000000d4;
  float fStack000000d8;
  float fStack000000dc;
  float fStack000000e0;
  float fStack000000e4;
  float fStack000000e8;
  float fStack000000ec;
  float in_stack_00000104;
  float in_stack_00000108;
  float in_stack_00000110;
  float in_stack_00000114;
  float in_stack_00000118;
  float in_stack_0000011c;
  float in_stack_00000120;
  float in_stack_00000124;
  float in_stack_00000128;
  float in_stack_0000012c;
  float in_stack_00000134;
  float in_stack_00000138;
  void *in_stack_00000204;
  
  iStack00000034 = in_EAX;
  do {
    iStack00000028 = 0;
    puStack00000038 = param_1;
    uStack00000040 = param_2;
    iStack00000044 = unaff_EDI;
    while( true ) {
      while( true ) {
        fVar6 = (float10)DAT_01ee3960;
        if (DAT_01ee3960 < 0) {
          fVar6 = fVar6 + (float10)_DAT_01810160;
        }
        fVar11 = (float)(in_ST0 / fVar6);
        fVar7 = in_stack_0000007c * fVar11 + in_XMM5_Da;
        fVar8 = (((fVar11 * in_stack_00000078 + in_XMM2_Da) - fVar7) * fVar11 + fVar7) *
                DAT_01816a8c * DAT_01890744;
        fVar7 = in_XMM3_Da * fVar11 + in_XMM1_Da;
        fStack000000d0 =
             (*(float *)(unaff_ESI + 0xa0) + *(float *)(unaff_ESI + 0x80)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x90) * fVar8 + *(float *)(unaff_ESI + 0x70);
        fStack000000d4 =
             (*(float *)(unaff_ESI + 0xa4) + *(float *)(unaff_ESI + 0x84)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x94) * fVar8 + *(float *)(unaff_ESI + 0x74);
        fStack000000d8 =
             (*(float *)(unaff_ESI + 0xa8) + *(float *)(unaff_ESI + 0x88)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x98) * fVar8 + *(float *)(unaff_ESI + 0x78);
        fStack000000dc =
             (*(float *)(unaff_ESI + 0xac) + *(float *)(unaff_ESI + 0x8c)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x9c) * fVar8 + *(float *)(unaff_ESI + 0x7c);
        fVar8 = fStack000000d0 * fStack000000d0 + fStack000000d8 * fStack000000d8 +
                fStack000000d4 * fStack000000d4;
        fStack000000b0 = fStack000000d8;
        fStack00000050 = fStack000000d0;
        fStack00000054 = fStack000000d4;
        if ((fVar8 != DAT_017f7ea0) &&
           (fStack000000b0 = in_XMM4_Da, fStack00000050 = in_XMM4_Da, fStack00000054 = in_XMM4_Da,
           DAT_01816ac0 <= fVar8)) {
          if ((DAT_01ee0d70 & 1) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 1;
            _DAT_01ee0d60 = DAT_01816a80;
            uRam01ee0d64 = 0;
            uRam01ee0d68 = 0;
            uRam01ee0d6c = 0;
          }
          if ((DAT_01ee0d70 & 2) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | param_2;
            _DAT_01ee0d50 = DAT_01816a8c;
            uRam01ee0d54 = 0;
            uRam01ee0d58 = 0;
            uRam01ee0d5c = 0;
          }
          auVar9 = rsqrtss(ZEXT416((uint)fVar8),ZEXT416((uint)fVar8));
          fVar10 = auVar9._0_4_;
          fVar8 = _DAT_01ee0d50 * fVar10 * (_DAT_01ee0d60 - fVar8 * fVar10 * fVar10);
          fStack000000b0 = fStack000000d8 * fVar8;
          fStack00000050 = fVar8 * fStack000000d0;
          fStack00000054 = fStack000000d4 * fVar8;
        }
        fVar11 = (((in_XMM0_Da * fVar11 + in_XMM7_Da) - fVar7) * fVar11 + fVar7) * DAT_01816a8c *
                 DAT_01890744;
        fStack000000a8 = fStack00000050;
        fStack000000ac = fStack00000054;
        fStack000000e0 =
             (*(float *)(unaff_ESI + 0xa0) + *(float *)(unaff_ESI + 0x70)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x90) * fVar11 + *(float *)(unaff_ESI + 0x80);
        fStack000000e4 =
             (*(float *)(unaff_ESI + 0xa4) + *(float *)(unaff_ESI + 0x74)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x94) * fVar11 + *(float *)(unaff_ESI + 0x84);
        fStack000000e8 =
             (*(float *)(unaff_ESI + 0xa8) + *(float *)(unaff_ESI + 0x78)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x98) * fVar11 + *(float *)(unaff_ESI + 0x88);
        fStack000000ec =
             (*(float *)(unaff_ESI + 0xac) + *(float *)(unaff_ESI + 0x7c)) * in_XMM4_Da +
             *(float *)(unaff_ESI + 0x9c) * fVar11 + *(float *)(unaff_ESI + 0x8c);
        fVar11 = fStack000000e0 * fStack000000e0 + fStack000000e8 * fStack000000e8 +
                 fStack000000e4 * fStack000000e4;
        fStack0000008c = fStack000000e8;
        fStack00000088 = fStack000000e4;
        fStack00000084 = fStack000000e0;
        if ((fVar11 != DAT_017f7ea0) &&
           (fStack0000008c = in_XMM4_Da, fStack00000088 = in_XMM4_Da, fStack00000084 = in_XMM4_Da,
           DAT_01816ac0 <= fVar11)) {
          if ((DAT_01ee0d70 & 1) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 1;
            _DAT_01ee0d60 = DAT_01816a80;
            uRam01ee0d64 = 0;
            uRam01ee0d68 = 0;
            uRam01ee0d6c = 0;
          }
          if ((DAT_01ee0d70 & 2) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | param_2;
            _DAT_01ee0d50 = DAT_01816a8c;
            uRam01ee0d54 = 0;
            uRam01ee0d58 = 0;
            uRam01ee0d5c = 0;
          }
          auVar9 = rsqrtss(ZEXT416((uint)fVar11),ZEXT416((uint)fVar11));
          fVar7 = auVar9._0_4_;
          fVar11 = _DAT_01ee0d50 * fVar7 * (_DAT_01ee0d60 - fVar11 * fVar7 * fVar7);
          fStack0000008c = fStack000000e8 * fVar11;
          fStack00000088 = fStack000000e4 * fVar11;
          fStack00000084 = fVar11 * fStack000000e0;
        }
        fStack00000090 = fStack0000008c * fStack00000054 - fStack00000088 * fStack000000b0;
        fStack00000094 = fStack000000b0 * fStack00000084 - fStack0000008c * fStack00000050;
        fStack00000098 = fStack00000088 * fStack00000050 - fStack00000054 * fStack00000084;
        fVar11 = fStack00000090 * fStack00000090 + fStack00000094 * fStack00000094 +
                 fStack00000098 * fStack00000098;
        fStack00000020 = fStack00000094;
        fStack00000024 = fStack00000098;
        fStack0000001c = fStack00000090;
        if ((fVar11 != DAT_017f7ea0) &&
           (fStack00000020 = in_XMM4_Da, fStack00000024 = in_XMM4_Da, fStack0000001c = in_XMM4_Da,
           DAT_01816ac0 <= fVar11)) {
          if ((DAT_01ee0d70 & 1) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | 1;
            _DAT_01ee0d60 = DAT_01816a80;
            uRam01ee0d64 = 0;
            uRam01ee0d68 = 0;
            uRam01ee0d6c = 0;
          }
          if ((DAT_01ee0d70 & 2) == 0) {
            DAT_01ee0d70 = DAT_01ee0d70 | param_2;
            _DAT_01ee0d50 = DAT_01816a8c;
            uRam01ee0d54 = 0;
            uRam01ee0d58 = 0;
            uRam01ee0d5c = 0;
          }
          auVar9 = rsqrtss(ZEXT416((uint)fVar11),ZEXT416((uint)fVar11));
          fVar7 = auVar9._0_4_;
          fVar11 = _DAT_01ee0d50 * fVar7 * (_DAT_01ee0d60 - fVar11 * fVar7 * fVar7);
          fStack00000020 = fVar11 * fStack00000094;
          fStack00000024 = fVar11 * fStack00000098;
          fStack0000001c = fVar11 * fStack00000090;
        }
        iVar3 = (*(int *)((int)in_stack_00000018 + 0x274) + 1) * iStack00000044 + iStack00000028 +
                in_stack_0000003c;
        fVar11 = in_XMM4_Da;
        if (iVar3 < *(int *)((int)in_stack_00000018 + 0x248)) {
          fVar11 = (float)(int)(*(byte *)(*(int *)((int)in_stack_00000018 + 0x244) + iVar3) - 0x7f)
                   * *(float *)((int)in_stack_00000018 + 0x250) * _DAT_01890748;
        }
        fStack000000c8 = fStack00000020 * fVar11;
        fStack000000cc = fStack00000024 * fVar11;
        iVar3 = *(int *)((int)in_stack_00000018 + 0x26c) + -1;
        if (unaff_EBX < 0) {
          iStack00000010 = 0;
        }
        else {
          iStack00000010 = unaff_EBX;
          if (iVar3 <= unaff_EBX) {
            iStack00000010 = iVar3;
          }
        }
        iVar3 = *(int *)((int)in_stack_00000018 + 0x270) + -1;
        if (iStack00000044 < 0) {
          iVar4 = 0;
        }
        else {
          iVar4 = iStack00000044;
          if (iVar3 <= iStack00000044) {
            iVar4 = iVar3;
          }
        }
        fVar7 = ((float)*(ushort *)
                         (*(int *)((int)in_stack_00000018 + 0x1d4) +
                         (*(int *)((int)in_stack_00000018 + 0x26c) * iVar4 + iStack00000010) * 2) -
                DAT_018199e8) * DAT_01890744;
        fVar10 = (float)iStack00000034;
        fVar8 = (float)(in_stack_00000070 + unaff_EBX);
        fStack000000a0 =
             *(float *)(unaff_ESI + 0x94) * fVar7 + *(float *)(unaff_ESI + 0x84) * fVar10 +
             *(float *)(unaff_ESI + 0x74) * fVar8 + *(float *)(unaff_ESI + 0xa4) + fStack000000c8;
        fStack0000009c =
             *(float *)(unaff_ESI + 0x90) * fVar7 + *(float *)(unaff_ESI + 0x80) * fVar10 +
             *(float *)(unaff_ESI + 0x70) * fVar8 + *(float *)(unaff_ESI + 0xa0) +
             fVar11 * fStack0000001c;
        fStack000000a4 =
             *(float *)(unaff_ESI + 0x98) * fVar7 + *(float *)(unaff_ESI + 0x88) * fVar10 +
             *(float *)(unaff_ESI + 0x78) * fVar8 + *(float *)(unaff_ESI + 0xa8) + fStack000000cc;
        param_1[-3] = CONCAT44(fStack000000a0,fStack0000009c);
        *(float *)(param_1 + -2) = fStack000000a4;
        *(ulonglong *)((int)param_1 + -0xc) = CONCAT44(fStack00000054,fStack00000050);
        *(float *)((int)param_1 + -4) = fStack000000b0;
        *param_1 = CONCAT44(fStack00000088,fStack00000084);
        *(float *)(param_1 + 1) = fStack0000008c;
        *(ulonglong *)((int)param_1 + 0xc) = CONCAT44(fStack00000020,fStack0000001c);
        *(float *)((int)param_1 + 0x14) = fStack00000024;
        iStack00000028 = iStack00000028 + 1;
        param_1 = param_1 + 0xc;
        unaff_EBX = unaff_EBX + 1;
        if (1 < iStack00000028) break;
        param_2 = 2;
      }
      iStack00000034 = iStack00000034 + 1;
      param_1 = puStack00000038 + 6;
      uStack00000040 = uStack00000040 - 1;
      puStack00000038 = param_1;
      if (uStack00000040 == 0) break;
      iStack00000044 = in_stack_00000080 + iStack00000034;
      param_2 = 2;
      iStack00000028 = 0;
      unaff_EBX = in_stack_0000003c;
    }
    puVar1 = *(undefined4 **)(unaff_EBP + 8);
    fStack00000030 = in_XMM5_Da;
    fStack00000048 = in_XMM7_Da;
    fStack0000004c = in_XMM2_Da;
    fStack00000068 = in_XMM0_Da;
    fStack0000006c = in_XMM1_Da;
    fStack00000074 = in_XMM3_Da;
    (**(code **)*puVar1)(&stack0x00000140,&stack0x00000170,&stack0x000001d0);
    (**(code **)*puVar1)(&stack0x000001c4,&stack0x00000194,&stack0x00000134);
    in_XMM4_Da = 0.0;
    do {
      in_stack_0000002c = in_stack_0000002c + 1;
      if (*(int *)(unaff_ESI + 0x2b4) <= in_stack_0000002c) {
        do {
          in_stack_00000014 = in_stack_00000014 + 1;
          if (*(int *)(unaff_ESI + 0x2b8) <= in_stack_00000014) {
            ExceptionList = in_stack_00000204;
            return;
          }
          in_stack_0000002c = 0;
        } while (*(int *)(unaff_ESI + 0x2b4) < 1);
      }
      iVar3 = *(int *)(unaff_ESI + 0x28);
      iVar5 = *(int *)(unaff_ESI + 0x2a8) + in_stack_00000014;
      unaff_EBX = *(int *)(unaff_ESI + 0x2a4) + in_stack_0000002c;
      iVar4 = *(int *)(iVar3 + 0x26c) + -1;
      if (unaff_EBX < 0) {
        iStack00000010 = 0;
      }
      else {
        iStack00000010 = unaff_EBX;
        if (iVar4 <= unaff_EBX) {
          iStack00000010 = iVar4;
        }
      }
      iVar4 = *(int *)(iVar3 + 0x270) + -1;
      if (iVar5 < 0) {
        iVar2 = 0;
      }
      else {
        iVar2 = iVar5;
        if (iVar4 <= iVar5) {
          iVar2 = iVar4;
        }
      }
    } while ((~*(byte *)(iVar2 * *(int *)(iVar3 + 0x26c) + *(int *)(iVar3 + 0x1e0) + iStack00000010)
             & 1) == 0);
    UTerrainComponent__unknown_00742630(in_stack_00000018,(int)&stack0x00000100,unaff_EBX,iVar5);
    in_XMM1_Da = in_stack_00000118 - in_stack_00000110;
    in_XMM2_Da = in_stack_00000128 - in_stack_00000108;
    in_stack_00000078 = (in_stack_00000138 - in_stack_00000118) - in_XMM2_Da;
    in_stack_00000070 = in_stack_0000002c - unaff_EBX;
    in_XMM5_Da = in_stack_00000124 - in_stack_00000104;
    in_stack_0000007c = (in_stack_00000134 - in_stack_00000114) - in_XMM5_Da;
    in_stack_00000080 = iVar5 - in_stack_00000014;
    in_XMM7_Da = in_stack_0000011c - in_stack_00000114;
    param_1 = (undefined8 *)&stack0x00000158;
    param_2 = 2;
    unaff_EDI = in_stack_00000080 + in_stack_00000014;
    in_XMM0_Da = (in_stack_0000012c - in_stack_00000124) - in_XMM7_Da;
    in_XMM3_Da = (in_stack_00000128 - in_stack_00000120) - in_XMM1_Da;
    iStack00000034 = in_stack_00000014;
    in_ST0 = extraout_ST0;
    in_stack_0000003c = unaff_EBX;
  } while( true );
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] DirSection virtual function #55
   VTable: vtable_DirSection at 017fc2d4 */

undefined4 __fastcall DirSection__vfunc_55(int param_1,int param_2)

{
  float fVar1;
  int iVar2;
  float *pfVar3;
  void *this;
  byte bVar4;
  float fVar5;
  uint uVar6;
  void *unaff_EBX;
  int unaff_EBP;
  bool in_ZF;
  float10 fVar7;
  float in_XMM0_Da;
  float in_XMM1_Da;
  float in_XMM6_Da;
  int in_stack_00000010;
  int iStack00000014;
  int iStack00000018;
  float in_stack_0000001c;
  float fStack00000020;
  float fStack00000024;
  int iStack0000002c;
  undefined4 uStack00000030;
  undefined4 uStack00000034;
  undefined4 uStack00000038;
  undefined4 uStack0000003c;
  undefined4 uStack00000040;
  undefined4 uStack00000044;
  float fStack00000048;
  undefined4 uStack0000004c;
  int iStack00000050;
  int iStack00000054;
  int iStack00000058;
  int iStack0000005c;
  int iStack00000060;
  int iStack00000064;
  undefined4 uStack00000068;
  int iStack0000006c;
  
  fVar5 = DAT_017f7ea0;
  if (!in_ZF) {
    iVar2 = *(int *)((int)unaff_EBX + 0x4c);
    fVar1 = *(float *)(iVar2 + 0x140);
    in_XMM1_Da = *(float *)(iVar2 + 0x144) * fVar1 * in_XMM1_Da;
    in_XMM0_Da = *(float *)(iVar2 + 0x148) * fVar1 * in_XMM0_Da;
    in_XMM6_Da = *(float *)(iVar2 + 0x14c) * fVar1 * in_XMM6_Da;
    in_stack_0000001c = in_XMM1_Da;
  }
  if (((float)((uint)(in_XMM1_Da - in_XMM0_Da) & DAT_01816ab0) < _DAT_018fde24) &&
     ((float)((uint)(in_XMM0_Da - in_XMM6_Da) & DAT_01816ab0) < _DAT_018fde24)) {
    pfVar3 = *(float **)(unaff_EBP + 0x18);
    if ((*pfVar3 != 0.0) || ((pfVar3[1] != 0.0 || (iStack00000018 = 1, pfVar3[2] != 0.0)))) {
      iStack00000018 = param_2;
    }
    this = *(void **)(unaff_EBP + 8);
    *(undefined4 *)((int)this + 0x24) = 0xffffffff;
    *(undefined4 *)((int)this + 0x40) = 0xffffffff;
    *(float *)((int)this + 0x20) = fVar5;
    *(int *)((int)this + 0x34) = param_2;
    *(int *)((int)this + 0x38) = param_2;
    uStack0000004c = 0xffffffff;
    uStack00000068 = 0xffffffff;
    *(int *)((int)this + 0x30) = param_2;
    *(int *)((int)this + 0x28) = param_2;
    *(int *)((int)this + 0x2c) = param_2;
    uStack00000030 = 0;
    uStack00000034 = 0;
    uStack00000038 = 0;
    uStack0000003c = 0;
    uStack00000040 = 0;
    uStack00000044 = 0;
    fStack00000048 = 0.0;
    iStack00000014 = 0;
    fStack00000020 = in_XMM0_Da;
    fStack00000024 = in_XMM6_Da;
    iStack0000002c = param_2;
    iStack00000050 = param_2;
    iStack00000054 = param_2;
    iStack00000058 = param_2;
    iStack0000005c = param_2;
    iStack00000060 = param_2;
    iStack00000064 = param_2;
    iStack0000006c = param_2;
    if (param_2 < *(int *)(param_1 + 0x44)) {
      do {
        iVar2 = *(int *)(*(int *)(param_1 + 0x40) + iStack00000014 * 4);
        if (iStack00000018 == param_2) {
          bVar4 = *(byte *)(iVar2 + 0x84) & 8;
        }
        else {
          bVar4 = *(byte *)(iVar2 + 0x84) & 4;
        }
        if (bVar4 != 0) {
          uVar6 = FUN_00608c10(unaff_EBX,*(uint *)(iVar2 + 0x7c),*(int *)(iVar2 + 0x80));
          if (uVar6 != 0xffffffff) {
            USkeletalMeshComponent__unknown_00608af0(unaff_EBX,(float *)&stack0x00000070,uVar6);
            fVar7 = FUN_005d0080((float *)&stack0x00000070);
            if ((float10)DAT_01816aa0 < ABS(fVar7)) {
              FUN_0060fe30(&stack0x00000070,DAT_01816ac0);
              fStack00000048 = DAT_017f7ea0;
              FUN_0092cc30((void *)(iVar2 + 0x48),(undefined4 *)&stack0x00000028,
                           (float *)&stack0x00000070,&stack0x0000001c,*(float **)(unaff_EBP + 0x14),
                           *(float **)(unaff_EBP + 0x10),*(float **)(unaff_EBP + 0x18),0);
              if (fStack00000048 < *(float *)((int)this + 0x20)) {
                FUN_0070e1f0(this,(undefined4 *)&stack0x00000028);
                *(int *)((int)this + 0x24) = iStack00000014;
                *(undefined4 *)((int)this + 0x34) = *(undefined4 *)(iVar2 + 0x7c);
                *(undefined4 *)((int)this + 0x38) = *(undefined4 *)(iVar2 + 0x80);
                *(void **)((int)this + 0x30) = unaff_EBX;
                *(undefined4 *)((int)this + 4) = *(undefined4 *)((int)unaff_EBX + 0x4c);
                *(undefined4 *)((int)this + 0x2c) =
                     *(undefined4 *)
                      (*(int *)(*(int *)(in_stack_00000010 + 0x40) + *(int *)((int)this + 0x24) * 4)
                      + 0x88);
              }
            }
          }
          param_2 = 0;
          param_1 = in_stack_00000010;
        }
        iStack00000014 = iStack00000014 + 1;
      } while (iStack00000014 < *(int *)(param_1 + 0x44));
    }
    if (*(float *)((int)this + 0x20) <= DAT_017f7ea0 && DAT_017f7ea0 != *(float *)((int)this + 0x20)
       ) {
      return 0;
    }
  }
  return 1;
}




/* ========== EntityManager.c ========== */

/*
 * SGW.exe - EntityManager (1 functions)
 */

void __thiscall EntityManager__vfunc_0(void *this,int param_1,undefined4 param_2,int *param_3)

{
  int iVar1;
  void *pvVar2;
  undefined4 *puVar3;
  int *piVar4;
  float fStack_30;
  float fStack_2c;
  float fStack_28;
  int aiStack_24 [3];
  exception aeStack_18 [12];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_017052a8;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (DAT_01ef2224 == 0) {
    ExceptionList = &pvStack_c;
    FUN_00c66840();
  }
  piVar4 = param_3;
  iVar1 = param_1;
  if (DAT_01ef2224 != 0) {
    FUN_00c66860();
    FUN_00c66880();
    FUN_00c66880();
    (**(code **)(*piVar4 + 8))();
    FUN_00c66880();
    FUN_00c66870();
  }
  if (*(int **)((int)this + 0xc) != (int *)0x0) {
    FUN_00dd1120(this,*(int **)((int)this + 0xc));
  }
  *(int *)((int)this + 0x14) = iVar1;
  pvVar2 = BW_client_entity_manager_7(iVar1,(short)param_2,0);
  fStack_30 = DAT_018cad90 * (float)PTR_017f94b8;
  fStack_2c = fStack_30;
  fStack_28 = fStack_30;
  FUN_0084a9d0(aiStack_24,0.0,0.0,0.0);
  FUN_00e68a10(pvVar2,(undefined8 *)&fStack_30,(undefined8 *)aiStack_24,0,0);
  puVar3 = BW__unknown_00c6c500((void *)((int)this + 0x24),&param_1);
  *puVar3 = pvVar2;
  FUN_00e68df0(pvVar2,1);
  *(void **)((int)this + 0xc) = pvVar2;
  FUN_00dd1e40(this,(int)pvVar2);
  *(undefined4 *)((int)this + 0x10) = 0;
  piVar4 = (int *)scalable_malloc(4);
  if (piVar4 == (int *)0x0) {
    FUN_004099f0(aeStack_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(aeStack_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  uStack_4 = 0;
  *piVar4 = iVar1;
  param_2 = 0;
  pvVar2 = (void *)thunk_FUN_0054c900();
  FUN_00dd4a90(pvVar2,0,piVar4,1);
  uStack_4 = 0xffffffff;
                    /* WARNING: Subroutine does not return */
  scalable_free(0);
}




/* ========== ErrorMessage.c ========== */

/*
 * SGW.exe - ErrorMessage (1 functions)
 */

undefined4 * __thiscall ErrorMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01586850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FListenHelper.c ========== */

/*
 * SGW.exe - FListenHelper (1 functions)
 */

undefined4 FListenHelper__vfunc_0(void)

{
  if ((DAT_01f11fe0 != 0) && (DAT_01f11fe8 < 1)) {
    return 0;
  }
  return 1;
}




/* ========== FNetObjectNotify.c ========== */

/*
 * SGW.exe - FNetObjectNotify (1 functions)
 */

/* Also in vtable: FNetObjectNotify (slot 0) */

undefined4 * __thiscall FNetObjectNotify__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FNetObjectNotify::vftable;
  if (DAT_01ec699c == this) {
    DAT_01ec699c = (void *)0x0;
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FNetworkActorCreate.c ========== */

/*
 * SGW.exe - FNetworkActorCreate (1 functions)
 */

void __thiscall FNetworkActorCreate__vfunc_0(void *this,int *param_1)

{
  int *piVar1;
  int *piVar2;
  void *this_00;
  void *this_01;
  int *piVar3;
  
  FUN_0047f0e0(param_1,(int)this + 0x20,4);
  FUN_0047f0e0(param_1,(int)this + 0x24,4);
  FUN_0047f0e0(param_1,(int)this + 0x28,4);
  FUN_0047f0e0(param_1,(int)this + 0x2c,4);
  FUN_0047f0e0(param_1,(int)this + 0x30,4);
  FUN_0047f0e0(param_1,(int)this + 0x34,4);
  piVar3 = (int *)((int)this + 0x38);
  piVar2 = (int *)((int)this + 0x14);
  piVar1 = FUN_00485df0(this_00,param_1,(int *)((int)this + 8));
  piVar2 = FUN_00485df0(this_01,piVar1,piVar2);
  FUN_004d3e20(piVar2,piVar3);
  return;
}




/* ========== FNetworkActorDelete.c ========== */

/*
 * SGW.exe - FNetworkActorDelete (1 functions)
 */

void __thiscall FNetworkActorDelete__vfunc_0(void *this,int *param_1)

{
  FUN_00485df0((void *)((int)this + 8),param_1,(void *)((int)this + 8));
  return;
}




/* ========== FNetworkActorMove.c ========== */

/*
 * SGW.exe - FNetworkActorMove (1 functions)
 */

void __thiscall FNetworkActorMove__vfunc_0(void *this,int *param_1)

{
  void *this_00;
  
  FUN_0047f0e0(param_1,(int)this + 0x14,4);
  FUN_0047f0e0(param_1,(int)this + 0x18,4);
  FUN_0047f0e0(param_1,(int)this + 0x1c,4);
  FUN_0047f0e0(param_1,(int)this + 0x20,4);
  FUN_0047f0e0(param_1,(int)this + 0x24,4);
  FUN_0047f0e0(param_1,(int)this + 0x28,4);
  FUN_00485df0(this_00,param_1,(int *)((int)this + 8));
  return;
}




/* ========== FNetworkObjectRename.c ========== */

/*
 * SGW.exe - FNetworkObjectRename (1 functions)
 */

void __thiscall FNetworkObjectRename__vfunc_0(void *this,void *param_1)

{
  int *piVar1;
  void *this_00;
  int *piVar2;
  
  piVar2 = (int *)((int)this + 0x14);
  piVar1 = FUN_00485df0(param_1,param_1,(int *)((int)this + 8));
  FUN_00485df0(this_00,piVar1,piVar2);
  return;
}




/* ========== FNetworkPropertyChange.c ========== */

/*
 * SGW.exe - FNetworkPropertyChange (1 functions)
 */

void __thiscall FNetworkPropertyChange__vfunc_0(void *this,int *param_1)

{
  int *piVar1;
  int *piVar2;
  void *this_00;
  void *this_01;
  int *this_02;
  
  FUN_0047f0e0(param_1,(int)this + 0x2c,4);
  this_02 = (int *)((int)this + 0x20);
  piVar2 = (int *)((int)this + 0x14);
  piVar1 = FUN_00485df0(this_02,param_1,(int *)((int)this + 8));
  piVar2 = FUN_00485df0(this_00,piVar1,piVar2);
  FUN_00485df0(this_01,piVar2,this_02);
  return;
}




/* ========== FNetworkRemoteConsoleCommand.c ========== */

/*
 * SGW.exe - FNetworkRemoteConsoleCommand (1 functions)
 */

void __thiscall FNetworkRemoteConsoleCommand__vfunc_0(void *this,int *param_1)

{
  FUN_00485df0((void *)((int)this + 8),param_1,(void *)((int)this + 8));
  return;
}




/* ========== FixedDictDataType.c ========== */

/*
 * SGW.exe - FixedDictDataType (1 functions)
 */

undefined4 * __thiscall FixedDictDataType__vfunc_0(void *this,byte param_1)

{
  FUN_015a0650(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FixedDictMetaDataType.c ========== */

/*
 * SGW.exe - FixedDictMetaDataType (1 functions)
 */

undefined4 * __thiscall FixedDictMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_015982f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== IFileSystem.c ========== */

/*
 * SGW.exe - IFileSystem (1 functions)
 */

/* Also in vtable: IFileSystem (slot 0) */

undefined4 * __thiscall IFileSystem__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = IFileSystem::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ListenerMessage.c ========== */

/*
 * SGW.exe - ListenerMessage (1 functions)
 */

undefined4 * __thiscall ListenerMessage__vfunc_0(void *this,byte param_1)

{
  FUN_015864b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MachineGuardMessage.c ========== */

/*
 * SGW.exe - MachineGuardMessage (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage */

undefined4 * __thiscall MachineGuardMessage__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = MachineGuardMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MachineGuardMessage_ReplyHandler.c ========== */

/*
 * SGW.exe - MachineGuardMessage_ReplyHandler (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage_ReplyHandler */

undefined4 * __thiscall MachineGuardMessage_ReplyHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = MachineGuardMessage::ReplyHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MachinedAnnounceMessage.c ========== */

/*
 * SGW.exe - MachinedAnnounceMessage (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage */

undefined4 * __thiscall MachinedAnnounceMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_017942f8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = MachinedAnnounceMessage::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this = MachineGuardMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== MemoryIStream.c ========== */

/*
 * SGW.exe - MemoryIStream (1 functions)
 */

undefined4 * __thiscall MemoryIStream__vfunc_0(void *this,byte param_1)

{
  FUN_0157af80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MemoryOStream.c ========== */

/*
 * SGW.exe - MemoryOStream (1 functions)
 */

undefined4 * __thiscall MemoryOStream__vfunc_0(void *this,byte param_1)

{
  FUN_00dd3d10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_BaseNub.c ========== */

/*
 * SGW.exe - Mercury_BaseNub (1 functions)
 */

undefined4 * __thiscall Mercury_BaseNub__vfunc_0(void *this,byte param_1)

{
  FUN_01577a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_BaseNub_ProcessMessageHandler.c ========== */

/*
 * SGW.exe - Mercury_BaseNub_ProcessMessageHandler (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage_ReplyHandler */

undefined4 * __thiscall Mercury_BaseNub_ProcessMessageHandler__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01793018;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = MachineGuardMessage::ReplyHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== Mercury_Bundle.c ========== */

/*
 * SGW.exe - Mercury_Bundle (1 functions)
 */

undefined4 * __thiscall Mercury_Bundle__vfunc_0(void *this,byte param_1)

{
  FUN_0157a2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_BundlePrimer.c ========== */

/*
 * SGW.exe - Mercury_BundlePrimer (1 functions)
 */

/* References Mercury RTTI vtable: BundlePrimer
   References Mercury RTTI vtable: vtable_Mercury_BundlePrimer */

undefined4 * __thiscall Mercury_BundlePrimer__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Mercury::BundlePrimer::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_Channel.c ========== */

/*
 * SGW.exe - Mercury_Channel (1 functions)
 */

undefined4 * __thiscall Mercury_Channel__vfunc_0(void *this,byte param_1)

{
  FUN_01576b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_ChannelInternal.c ========== */

/*
 * SGW.exe - Mercury_ChannelInternal (1 functions)
 */

undefined4 * __thiscall Mercury_ChannelInternal__vfunc_0(void *this,byte param_1)

{
  FUN_0158d190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_ClientChannelRegMessage.c ========== */

/*
 * SGW.exe - Mercury_ClientChannelRegMessage (1 functions)
 */

/* References Mercury RTTI vtable: ClientMessage */

undefined4 * __thiscall Mercury_ClientChannelRegMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01794da0;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = Mercury::ClientMessage::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::
       CountedBaseTempl<struct_tbb::atomic<int>,struct_CME::Detail::DefaultCountTypeTraits<struct_tbb::atomic<int>,int>_>
       ::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== Mercury_ClientExceptionMessage.c ========== */

/*
 * SGW.exe - Mercury_ClientExceptionMessage (1 functions)
 */

undefined4 * __thiscall Mercury_ClientExceptionMessage__vfunc_0(void *this,byte param_1)

{
  FUN_0158d620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_ClientIncomingMessage.c ========== */

/*
 * SGW.exe - Mercury_ClientIncomingMessage (1 functions)
 */

undefined4 * __thiscall Mercury_ClientIncomingMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01794d58;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_0158d510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== Mercury_ClientMessage.c ========== */

/*
 * SGW.exe - Mercury_ClientMessage (1 functions)
 */

/* References Mercury RTTI vtable: ClientMessage */

undefined4 * __thiscall Mercury_ClientMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01794c48;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = Mercury::ClientMessage::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::
       CountedBaseTempl<struct_tbb::atomic<int>,struct_CME::Detail::DefaultCountTypeTraits<struct_tbb::atomic<int>,int>_>
       ::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== Mercury_ClientNetMessage.c ========== */

/*
 * SGW.exe - Mercury_ClientNetMessage (1 functions)
 */

undefined4 * __thiscall Mercury_ClientNetMessage__vfunc_0(void *this,byte param_1)

{
  FUN_0158d510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_InputMessageHandler.c ========== */

/*
 * SGW.exe - Mercury_InputMessageHandler (1 functions)
 */

/* References Mercury RTTI vtable: InputMessageHandler */

undefined4 * __thiscall Mercury_InputMessageHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Mercury::InputMessageHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_Nub.c ========== */

/*
 * SGW.exe - Mercury_Nub (1 functions)
 */

undefined4 * __thiscall Mercury_Nub__vfunc_0(void *this,byte param_1)

{
  FUN_01583170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_NubException.c ========== */

/*
 * SGW.exe - Mercury_NubException (1 functions)
 */

/* References Mercury RTTI vtable: NubException
   References Mercury RTTI vtable: vtable_Mercury_NubException */

undefined4 * __thiscall Mercury_NubException__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Mercury::NubException::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_Nub_Connection.c ========== */

/*
 * SGW.exe - Mercury_Nub_Connection (1 functions)
 */

int __thiscall Mercury_Nub_Connection__vfunc_0(void *this,byte param_1)

{
  FUN_015830f0((int)this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return (int)this;
}




/* ========== Mercury_Nub_ReplyHandlerElement.c ========== */

/*
 * SGW.exe - Mercury_Nub_ReplyHandlerElement (1 functions)
 */

/* References Mercury RTTI vtable: TimerExpiryHandler */

undefined4 * __thiscall Mercury_Nub_ReplyHandlerElement__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01793528;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = Mercury::TimerExpiryHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== Mercury_Packet.c ========== */

/*
 * SGW.exe - Mercury_Packet (1 functions)
 */

undefined4 * __thiscall Mercury_Packet__vfunc_0(void *this,byte param_1)

{
  FUN_0158a340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_PacketFilter.c ========== */

/*
 * SGW.exe - Mercury_PacketFilter (1 functions)
 */

/* References Mercury RTTI vtable: PacketFilter
   References Mercury RTTI vtable: vtable_Mercury_PacketFilter */

undefined4 * __thiscall Mercury_PacketFilter__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  uint uVar2;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_017982a0;
  local_c = ExceptionList;
  uVar2 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  *(undefined ***)this = Mercury::PacketFilter::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this = ReferenceCount::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar2);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== Mercury_ReplyMessageHandler.c ========== */

/*
 * SGW.exe - Mercury_ReplyMessageHandler (1 functions)
 */

/* References Mercury RTTI vtable: ReplyMessageHandler */

undefined4 * __thiscall Mercury_ReplyMessageHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Mercury::ReplyMessageHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mercury_TimerExpiryHandler.c ========== */

/*
 * SGW.exe - Mercury_TimerExpiryHandler (1 functions)
 */

/* References Mercury RTTI vtable: TimerExpiryHandler */

undefined4 * __thiscall Mercury_TimerExpiryHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Mercury::TimerExpiryHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MetaDataType.c ========== */

/*
 * SGW.exe - MetaDataType (1 functions)
 */

undefined4 * __thiscall MetaDataType__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = MetaDataType::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Moo_IndicesHolder.c ========== */

/*
 * SGW.exe - Moo_IndicesHolder (1 functions)
 */

undefined4 * __thiscall Moo_IndicesHolder__vfunc_0(void *this,byte param_1)

{
  FUN_01173c40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Moo_IndicesReference.c ========== */

/*
 * SGW.exe - Moo_IndicesReference (1 functions)
 */

undefined4 * __thiscall Moo_IndicesReference__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = Moo::IndicesReference::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Moo_Node.c ========== */

/*
 * SGW.exe - Moo_Node (1 functions)
 */

undefined4 * __thiscall Moo_Node__vfunc_0(void *this,byte param_1)

{
  FUN_01575b30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MultiFileSystem.c ========== */

/*
 * SGW.exe - MultiFileSystem (12 functions)
 */

/* [VTable] MultiFileSystem virtual function #1
   VTable: vtable_MultiFileSystem at 017fc404 */

int __thiscall MultiFileSystem__vfunc_1(void *this,undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  int iVar2;
  code *pcVar3;
  undefined4 *puVar4;
  
  pcVar3 = _invalid_parameter_noinfo_exref;
  puVar4 = *(undefined4 **)((int)this + 8);
  if (*(undefined4 **)((int)this + 0xc) < puVar4) {
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      (*pcVar3)();
      pcVar3 = _invalid_parameter_noinfo_exref;
    }
    if (this == (void *)0xfffffffc) {
      (*pcVar3)();
      pcVar3 = _invalid_parameter_noinfo_exref;
    }
    if (puVar4 == puVar1) break;
    if (this == (void *)0xfffffffc) {
      (*pcVar3)();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar4) {
      (*pcVar3)();
    }
    iVar2 = (**(code **)(*(int *)*puVar4 + 4))(param_1,param_2);
    if (iVar2 != 0) {
      return iVar2;
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar4) {
      (*pcVar3)();
    }
    puVar4 = puVar4 + 1;
  }
  return 0;
}


/* [VTable] MultiFileSystem virtual function #5
   VTable: vtable_MultiFileSystem at 017fc404 */

undefined4 __thiscall MultiFileSystem__vfunc_5(void *this,undefined4 param_1)

{
  undefined4 *puVar1;
  uint in_EAX;
  uint extraout_EAX;
  code *pcVar2;
  undefined4 *puVar3;
  
  pcVar2 = _invalid_parameter_noinfo_exref;
  puVar3 = *(undefined4 **)((int)this + 8);
  if (*(undefined4 **)((int)this + 0xc) < puVar3) {
    _invalid_parameter_noinfo();
    in_EAX = extraout_EAX;
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      in_EAX = (*pcVar2)();
      pcVar2 = _invalid_parameter_noinfo_exref;
    }
    if (this == (void *)0xfffffffc) {
      in_EAX = (*pcVar2)();
      pcVar2 = _invalid_parameter_noinfo_exref;
    }
    if (puVar3 == puVar1) break;
    if (this == (void *)0xfffffffc) {
      (*pcVar2)();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      (*pcVar2)();
    }
    in_EAX = (**(code **)(*(int *)*puVar3 + 0x14))(param_1);
    if ((char)in_EAX != '\0') {
      return CONCAT31((int3)(in_EAX >> 8),1);
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      in_EAX = (*pcVar2)();
    }
    puVar3 = puVar3 + 1;
  }
  return in_EAX & 0xffffff00;
}


/* [VTable] MultiFileSystem virtual function #7
   VTable: vtable_MultiFileSystem at 017fc404 */

undefined4 __thiscall MultiFileSystem__vfunc_7(void *this,undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  uint in_EAX;
  uint extraout_EAX;
  code *pcVar2;
  undefined4 *puVar3;
  
  pcVar2 = _invalid_parameter_noinfo_exref;
  puVar3 = *(undefined4 **)((int)this + 8);
  if (*(undefined4 **)((int)this + 0xc) < puVar3) {
    _invalid_parameter_noinfo();
    in_EAX = extraout_EAX;
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      in_EAX = (*pcVar2)();
      pcVar2 = _invalid_parameter_noinfo_exref;
    }
    if (this == (void *)0xfffffffc) {
      in_EAX = (*pcVar2)();
      pcVar2 = _invalid_parameter_noinfo_exref;
    }
    if (puVar3 == puVar1) break;
    if (this == (void *)0xfffffffc) {
      (*pcVar2)();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      (*pcVar2)();
    }
    in_EAX = (**(code **)(*(int *)*puVar3 + 0x1c))(param_1,param_2);
    if ((char)in_EAX != '\0') {
      return CONCAT31((int3)(in_EAX >> 8),1);
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      in_EAX = (*pcVar2)();
    }
    puVar3 = puVar3 + 1;
  }
  return in_EAX & 0xffffff00;
}


/* [VTable] MultiFileSystem virtual function #8
   VTable: vtable_MultiFileSystem at 017fc404 */

undefined4 __thiscall MultiFileSystem__vfunc_8(void *this,undefined4 param_1)

{
  undefined4 *puVar1;
  uint in_EAX;
  uint extraout_EAX;
  code *pcVar2;
  undefined4 *puVar3;
  
  pcVar2 = _invalid_parameter_noinfo_exref;
  puVar3 = *(undefined4 **)((int)this + 8);
  if (*(undefined4 **)((int)this + 0xc) < puVar3) {
    _invalid_parameter_noinfo();
    in_EAX = extraout_EAX;
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      in_EAX = (*pcVar2)();
      pcVar2 = _invalid_parameter_noinfo_exref;
    }
    if (this == (void *)0xfffffffc) {
      in_EAX = (*pcVar2)();
      pcVar2 = _invalid_parameter_noinfo_exref;
    }
    if (puVar3 == puVar1) break;
    if (this == (void *)0xfffffffc) {
      (*pcVar2)();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      (*pcVar2)();
    }
    in_EAX = (**(code **)(*(int *)*puVar3 + 0x20))(param_1);
    if ((char)in_EAX != '\0') {
      return CONCAT31((int3)(in_EAX >> 8),1);
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      in_EAX = (*pcVar2)();
    }
    puVar3 = puVar3 + 1;
  }
  return in_EAX & 0xffffff00;
}


/* [VTable] MultiFileSystem virtual function #9
   VTable: vtable_MultiFileSystem at 017fc404 */

int __thiscall MultiFileSystem__vfunc_9(void *this,undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 *puVar3;
  
  puVar3 = *(undefined4 **)((int)this + 8);
  if (*(undefined4 **)((int)this + 0xc) < puVar3) {
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      _invalid_parameter_noinfo();
    }
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (puVar3 == puVar1) break;
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      _invalid_parameter_noinfo();
    }
    iVar2 = (**(code **)(*(int *)*puVar3 + 0x24))(param_1,param_2);
    if (iVar2 != 0) {
      return iVar2;
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      _invalid_parameter_noinfo();
    }
    puVar3 = puVar3 + 1;
  }
  return 0;
}


/* [VTable] MultiFileSystem virtual function #2
   VTable: vtable_MultiFileSystem at 017fc404 */

int __thiscall MultiFileSystem__vfunc_2(void *this,int param_1,uint param_2,undefined4 param_3)

{
  int iVar1;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167b0e9;
  pvStack_c = ExceptionList;
  iVar1 = *(int *)((int)this + 8);
  if ((iVar1 == 0) || (*(int *)((int)this + 0xc) - iVar1 >> 2 == 0)) {
    ExceptionList = &pvStack_c;
    *(undefined4 *)(param_1 + 0x18) = 0xf;
    param_2 = param_2 & 0xffffff00;
    *(undefined4 *)(param_1 + 0x14) = 0;
    std::char_traits<char>::assign((char *)(param_1 + 4),(char *)&param_2);
  }
  else {
    if ((iVar1 == 0) || (ExceptionList = &pvStack_c, *(int *)((int)this + 0xc) - iVar1 >> 2 == 0)) {
      ExceptionList = &pvStack_c;
      _invalid_parameter_noinfo();
    }
    (**(code **)(*(int *)**(undefined4 **)((int)this + 8) + 8))(param_1,param_2,param_3);
  }
  ExceptionList = pvStack_c;
  return param_1;
}


/* WARNING: Removing unreachable block (ram,0x004620f0) */
/* WARNING: Removing unreachable block (ram,0x004620fc) */
/* WARNING: Removing unreachable block (ram,0x00462100) */
/* [VTable] MultiFileSystem virtual function #4
   VTable: vtable_MultiFileSystem at 017fc404 */

undefined4 * __thiscall MultiFileSystem__vfunc_4(void *this,undefined4 *param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 *puVar3;
  undefined4 *local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167bf51;
  local_c = ExceptionList;
  local_10 = 0;
  puVar3 = *(undefined4 **)((int)this + 8);
  ExceptionList = &local_c;
  if (*(undefined4 **)((int)this + 0xc) < puVar3) {
    ExceptionList = &local_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      _invalid_parameter_noinfo();
    }
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (puVar3 == puVar1) {
      *param_1 = 0;
      ExceptionList = local_c;
      return param_1;
    }
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      _invalid_parameter_noinfo();
    }
    (**(code **)(*(int *)*puVar3 + 0x10))(&local_14,param_2);
    local_4 = 1;
    if (local_14 != (undefined4 *)0x0) break;
    local_4 = 0;
    local_14 = (undefined4 *)0x0;
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      _invalid_parameter_noinfo();
    }
    puVar3 = puVar3 + 1;
  }
  *param_1 = local_14;
  FUN_00457e40((int)local_14);
  puVar3 = local_14;
  local_10 = 1;
  local_4 = 0;
  if (((local_14 != (undefined4 *)0x0) && (iVar2 = FUN_00457e50((int)local_14), iVar2 == 1)) &&
     (puVar3 != (undefined4 *)0x0)) {
    (**(code **)*puVar3)(1);
  }
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] MultiFileSystem virtual function #6
   VTable: vtable_MultiFileSystem at 017fc404 */

undefined4 __thiscall MultiFileSystem__vfunc_6(void *this,undefined4 param_1,undefined4 *param_2)

{
  uint uVar1;
  undefined4 *puVar2;
  int *piVar3;
  uint extraout_EAX;
  int iVar4;
  uint uVar5;
  undefined4 *puVar6;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_0167bf88;
  local_c = ExceptionList;
  local_4 = 0;
  puVar6 = *(undefined4 **)((int)this + 8);
  uVar1 = (int)this + 4;
  ExceptionList = &local_c;
  if (*(undefined4 **)((int)this + 0xc) < puVar6) {
    ExceptionList = &local_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar2 = *(undefined4 **)((int)this + 0xc);
    if (puVar2 < *(undefined4 **)((int)this + 8)) {
      _invalid_parameter_noinfo();
    }
    uVar5 = uVar1;
    if (uVar1 == 0) {
      _invalid_parameter_noinfo();
      uVar5 = extraout_EAX;
    }
    if (puVar6 == puVar2) {
      local_4 = 0xffffffff;
      if ((param_2 != (undefined4 *)0x0) && (uVar5 = FUN_00457e50((int)param_2), uVar5 == 1)) {
        uVar5 = (**(code **)*param_2)();
      }
      ExceptionList = local_c;
      return uVar5 & 0xffffff00;
    }
    if (uVar1 == 0) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar6) {
      _invalid_parameter_noinfo();
    }
    piVar3 = (int *)*puVar6;
    if (param_2 != (undefined4 *)0x0) {
      FUN_00457e40((int)param_2);
    }
    local_4 = local_4 & 0xffffff00;
    iVar4 = (**(code **)(*piVar3 + 0x18))(param_1);
    if ((char)iVar4 != '\0') break;
    if (*(undefined4 **)((int)this + 0xc) <= puVar6) {
      _invalid_parameter_noinfo();
    }
    puVar6 = puVar6 + 1;
  }
  local_4 = 0xffffffff;
  if ((param_2 != (undefined4 *)0x0) && (iVar4 = FUN_00457e50((int)param_2), iVar4 == 1)) {
    iVar4 = (**(code **)*param_2)();
  }
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)iVar4 >> 8),1);
}


/* Also in vtable: MultiFileSystem (slot 0) */

undefined4 * __thiscall MultiFileSystem__vfunc_0(void *this,byte param_1)

{
  FUN_00462480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] MultiFileSystem virtual function #10
   VTable: vtable_MultiFileSystem at 017fc404 */

void __fastcall MultiFileSystem__vfunc_10(int param_1)

{
  void *this;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this = (void *)scalable_malloc(0x14);
  if (this == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_004627f0(this,param_1);
  ExceptionList = local_c;
  return;
}


/* [VTable] MultiFileSystem virtual function #3
   VTable: vtable_MultiFileSystem at 017fc404 */

void * __thiscall MultiFileSystem__vfunc_3(void *this,void *param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  void *pvVar2;
  void *pvVar3;
  void *pvVar4;
  undefined1 *puVar5;
  undefined4 *local_20;
  undefined1 local_1c [4];
  void *pvStack_18;
  void *pvStack_14;
  undefined4 uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_0167c031;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined4 *)((int)param_1 + 4) = 0;
  *(undefined4 *)((int)param_1 + 8) = 0;
  *(undefined4 *)((int)param_1 + 0xc) = 0;
  local_4 = 0;
  local_20 = *(undefined4 **)((int)this + 8);
  if (*(undefined4 **)((int)this + 0xc) < local_20) {
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      _invalid_parameter_noinfo();
    }
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (local_20 == puVar1) {
      ExceptionList = local_c;
      return param_1;
    }
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)((int)this + 0xc) <= local_20) {
      _invalid_parameter_noinfo();
    }
    (**(code **)(*(int *)*local_20 + 0xc))(local_1c,param_2);
    pvVar3 = pvStack_14;
    local_4 = 1;
    if (pvStack_14 < pvStack_18) {
      _invalid_parameter_noinfo();
    }
    pvVar4 = pvStack_18;
    if (pvStack_14 < pvStack_18) {
      _invalid_parameter_noinfo();
    }
    pvVar2 = *(void **)((int)param_1 + 8);
    if (pvVar2 < *(void **)((int)param_1 + 4)) {
      _invalid_parameter_noinfo();
    }
    FUN_004629d0(param_1,param_1,pvVar2,local_1c,pvVar4,local_1c,pvVar3);
    local_4 = local_4 & 0xffffff00;
    if (pvStack_18 != (void *)0x0) break;
    pvStack_18 = (void *)0x0;
    pvStack_14 = (void *)0x0;
    uStack_10 = 0;
    if (*(undefined4 **)((int)this + 0xc) <= local_20) {
      _invalid_parameter_noinfo();
    }
    local_20 = local_20 + 1;
  }
  puVar5 = local_1c;
  pvVar3 = pvStack_18;
  pvVar4 = pvStack_14;
  WinFileSystem__unknown_00459570((uint)pvStack_18,(uint)pvStack_14);
                    /* WARNING: Subroutine does not return */
  scalable_free(pvStack_18,pvVar3,pvVar4,puVar5,param_1);
}


/* [VTable] MultiFileSystem virtual function #11
   VTable: vtable_MultiFileSystem at 017fc404 */

void __thiscall MultiFileSystem__vfunc_11(void *this,undefined4 param_1,void *param_2)

{
  undefined4 *puVar1;
  int iVar2;
  undefined4 *puVar3;
  undefined4 *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167c050;
  local_c = ExceptionList;
  puVar3 = *(undefined4 **)((int)this + 8);
  ExceptionList = &local_c;
  local_10 = this;
  if (*(undefined4 **)((int)this + 0xc) < puVar3) {
    ExceptionList = &local_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    puVar1 = *(undefined4 **)((int)this + 0xc);
    if (puVar1 < *(undefined4 **)((int)this + 8)) {
      _invalid_parameter_noinfo();
    }
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (puVar3 == puVar1) break;
    if (this == (void *)0xfffffffc) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      _invalid_parameter_noinfo();
    }
    (**(code **)(*(int *)*puVar3 + 0x10))(&local_10,param_1);
    uStack_4 = 0;
    if (local_10 != (undefined4 *)0x0) {
      FUN_0046a880(param_2,(int *)&local_10);
    }
    puVar1 = local_10;
    uStack_4 = 0xffffffff;
    if (((local_10 != (undefined4 *)0x0) && (iVar2 = FUN_00457e50((int)local_10), iVar2 == 1)) &&
       (puVar1 != (undefined4 *)0x0)) {
      (**(code **)*puVar1)(1);
    }
    local_10 = (undefined4 *)0x0;
    if (*(undefined4 **)((int)this + 0xc) <= puVar3) {
      _invalid_parameter_noinfo();
    }
    puVar3 = puVar3 + 1;
  }
  ExceptionList = local_c;
  return;
}




/* ========== NetworkEvent.c ========== */

/*
 * SGW.exe - NetworkEvent (2 functions)
 */

/* [VTable] NetworkEvent virtual function #3
   VTable: vtable_NetworkEvent at 017fb990 */

void __thiscall NetworkEvent__vfunc_3(void *this,void *param_1)

{
  FUN_00a372f0(param_1,*(uint *)((int)this + 8),this,&TypeDescriptor_01dab4cc,
               (type_info *)&NetworkEvent::RTTI_Type_Descriptor);
  return;
}


/* [VTable] NetworkEvent virtual function #2
   VTable: vtable_NetworkEvent at 017fb990 */

void __thiscall NetworkEvent__vfunc_2(void *this,void *param_1,undefined4 param_2)

{
  FUN_004414d0(param_1,*(int *)((int)this + 8),this,param_2);
  return;
}




/* ========== NullCollisionVisitor.c ========== */

/*
 * SGW.exe - NullCollisionVisitor (1 functions)
 */

undefined4 * __thiscall NullCollisionVisitor__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01792978;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = CollisionVisitor::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== PackedSection.c ========== */

/*
 * SGW.exe - PackedSection (21 functions)
 */

/* [VTable] PackedSection virtual function #1
   VTable: vtable_PackedSection at 017fdb9c */

int __fastcall PackedSection__vfunc_1(int param_1)

{
  if (*(short **)(param_1 + 0x1c) != (short *)0x0) {
    return (int)**(short **)(param_1 + 0x1c);
  }
  if ((*(int *)(param_1 + 0x14) == 0x30000000) && (*(int *)(param_1 + 0xc) == 0x30)) {
    return 4;
  }
  return 0;
}


/* [VTable] PackedSection virtual function #19
   VTable: vtable_PackedSection at 017fdb9c */

void __fastcall PackedSection__vfunc_19(int *param_1)

{
                    /* WARNING: Could not recover jumptable at 0x00473aa5. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*param_1 + 0x48))();
  return;
}


/* [VTable] PackedSection virtual function #21
   VTable: vtable_PackedSection at 017fdb9c */

void __thiscall PackedSection__vfunc_21(void *this,double param_1)

{
  (**(code **)(*(int *)this + 0x50))((float)param_1);
  return;
}


/* [VTable] PackedSection virtual function #12
   VTable: vtable_PackedSection at 017fdb9c */

int __fastcall PackedSection__vfunc_12(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 0xc);
  if (iVar1 < *(int *)(param_1 + 0x18)) {
    iVar1 = *(int *)(param_1 + 0x18);
  }
  return iVar1;
}


/* [VTable] PackedSection virtual function #13
   VTable: vtable_PackedSection at 017fdb9c
   [String discovery] References: "PackedSection::save" */

uint PackedSection__vfunc_13(int param_1)

{
  uint uVar1;
  
  if (0xf < *(uint *)(param_1 + 0x18)) {
    uVar1 = _00___FRawStaticIndexBuffer__vfunc_0();
    return uVar1 & 0xffffff00;
  }
  uVar1 = _00___FRawStaticIndexBuffer__vfunc_0();
  return uVar1 & 0xffffff00;
}


/* [VTable] PackedSection virtual function #5
   VTable: vtable_PackedSection at 017fdb9c */

void PackedSection__vfunc_5(undefined4 *param_1)

{
  *param_1 = 0;
  return;
}


/* [VTable] PackedSection virtual function #23
   VTable: vtable_PackedSection at 017fdb9c */

undefined1 * __thiscall
PackedSection__vfunc_23(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  undefined1 *puVar1;
  int iVar2;
  undefined4 uStack_4c;
  undefined4 local_48;
  undefined1 auStack_44 [4];
  char local_40 [4];
  undefined4 uStack_3c;
  uint uStack_38;
  undefined4 local_30;
  undefined4 local_2c;
  undefined1 auStack_28 [8];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined1 uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167db39;
  pvStack_c = ExceptionList;
  local_48 = 0;
  local_2c = 0xf;
  uStack_4c = (uint)(uint3)uStack_4c;
  local_30 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_40,(char *)((int)&uStack_4c + 3));
  uStack_4 = 1;
  iVar2 = (**(code **)(*(int *)this + 0x58))(auStack_28,auStack_44,param_3);
  puVar1 = puStack_8;
  uStack_10 = 2;
  FUN_0046ed40(puStack_8,iVar2);
  uStack_10 = 1;
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_30);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)&local_30,&stack0x00000000);
  uStack_10 = 0;
  if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_4c);
  }
  uStack_38 = 0xf;
  uStack_3c = 0;
  std::char_traits<char>::assign((char *)&uStack_4c,&stack0x00000000);
  ExceptionList = pvStack_18;
  return puVar1;
}


/* [VTable] PackedSection virtual function #11
   VTable: vtable_PackedSection at 017fdb9c */

void * __thiscall PackedSection__vfunc_11(void *this,void *param_1)

{
  char *pcVar1;
  uint uVar2;
  char local_11 [5];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01762009;
  pvStack_c = ExceptionList;
  local_11[1] = '\0';
  local_11[2] = '\0';
  local_11[3] = '\0';
  local_11[4] = '\0';
  pcVar1 = *(char **)((int)this + 8);
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  local_11[0] = '\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<char>::assign((char *)((int)param_1 + 4),local_11);
  uVar2 = std::char_traits<char>::length(pcVar1);
  FUN_004377d0(param_1,pcVar1,uVar2);
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] PackedSection virtual function #17
   VTable: vtable_PackedSection at 017fdb9c
   [String discovery] References: "PackedSection::asBool" */

bool __fastcall PackedSection__vfunc_17(int *param_1)

{
  undefined1 uVar1;
  int *unaff_EBX;
  uint uVar2;
  int *piVar3;
  int local_48 [6];
  undefined4 auStack_30 [2];
  undefined1 local_28 [8];
  void *pvStack_20;
  uint uStack_1c;
  uint uStack_18;
  char acStack_10 [4];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167dc98;
  local_c = ExceptionList;
  if (param_1[5] == 0x40000000) {
    return 0 < param_1[3];
  }
  ExceptionList = &local_c;
  PackedSection__unknown_00474320(local_48);
  local_4 = 0;
  FUN_00466c20((int)local_28);
  local_4 = CONCAT31(local_4._1_3_,1);
  uVar2 = 0;
  (**(code **)(*param_1 + 0x58))();
  acStack_10[0] = '\x03';
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  piVar3 = (int *)(uVar2 & 0xffffff);
  pvStack_20 = (void *)0x0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffab);
  (**(code **)(*unaff_EBX + 0x8c))(&stack0xffffffb0);
  _00___FRawStaticIndexBuffer__vfunc_0();
  uVar1 = (**(code **)(*piVar3 + 0x44))(local_c);
  uStack_18 = uStack_18 & 0xffffff00;
  if (0xf < (uint)local_48[2]) {
                    /* WARNING: Subroutine does not return */
    scalable_free(unaff_EBX);
  }
  local_48[2] = 0xf;
  acStack_10[0] = '\0';
  local_48[1] = 0;
  std::char_traits<char>::assign(&stack0xffffffac,acStack_10);
  uStack_18 = 0xffffffff;
  FUN_004585a0((int *)&stack0xffffffa4);
  ExceptionList = pvStack_20;
  return (bool)uVar1;
}


/* [VTable] PackedSection virtual function #18
   VTable: vtable_PackedSection at 017fdb9c
   [String discovery] References: "PackedSection::asInt" */

int __fastcall PackedSection__vfunc_18(int *param_1)

{
  void **ppvVar1;
  int iVar2;
  undefined4 uVar3;
  undefined4 unaff_ESI;
  int *unaff_EDI;
  undefined4 uStack_5c;
  int *local_48;
  undefined1 local_44 [4];
  undefined4 uStack_40;
  uint uStack_3c;
  undefined4 auStack_34 [3];
  undefined1 local_28 [4];
  undefined4 uStack_24;
  void *pvStack_20;
  undefined4 uStack_18;
  undefined1 uStack_14;
  undefined1 uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167dcf8;
  local_c = ExceptionList;
  ppvVar1 = &local_c;
  if (param_1[5] == 0x20000000) {
    switch(param_1[3]) {
    case 0:
      return 0;
    case 1:
      return (int)*(char *)param_1[4];
    case 2:
      return (int)*(short *)param_1[4];
    default:
      uStack_5c = 0x4746e9;
      ExceptionList = &local_c;
      _00___FRawStaticIndexBuffer__vfunc_0();
      ppvVar1 = ExceptionList;
      break;
    case 4:
      return *(int *)param_1[4];
    }
  }
  ExceptionList = ppvVar1;
  uStack_5c = param_1[5];
  _00___FRawStaticIndexBuffer__vfunc_0();
  PackedSection__unknown_00474320((int *)&local_48);
  local_4 = 0;
  iVar2 = FUN_00466c20((int)local_28);
  local_4 = CONCAT31(local_4._1_3_,1);
  uStack_5c = 0;
  uVar3 = (**(code **)(*param_1 + 0x58))(local_44,iVar2);
  uStack_10 = 2;
  (**(code **)(*local_48 + 0x8c))(uVar3);
  uStack_14 = 1;
  if (0xf < uStack_3c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(unaff_ESI);
  }
  uStack_3c = 0xf;
  uStack_5c = uStack_5c & 0xffffff;
  uStack_40 = 0;
  std::char_traits<char>::assign(&stack0xffffffb0,(char *)((int)&uStack_5c + 3));
  uStack_14 = 0;
  if (0xf < pvStack_20) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_34[0]);
  }
  pvStack_20 = (void *)0xf;
  uStack_5c = uStack_5c & 0xffffff;
  uStack_24 = 0;
  std::char_traits<char>::assign((char *)auStack_34,(char *)((int)&uStack_5c + 3));
  iVar2 = (**(code **)(*unaff_EDI + 0x48))(local_c);
  uStack_18 = 0xffffffff;
  FUN_004585a0(&uStack_5c);
  ExceptionList = pvStack_20;
  return iVar2;
}


/* [VTable] PackedSection virtual function #20
   VTable: vtable_PackedSection at 017fdb9c
   [String discovery] References: "PackedSection::asFloat" */

float10 __thiscall PackedSection__vfunc_20(int *param_1,float param_2)

{
  int *piVar1;
  undefined4 uVar2;
  int iVar3;
  float10 fVar4;
  double dVar5;
  int *local_48;
  undefined1 local_44 [4];
  char **local_40 [2];
  undefined1 auStack_38 [12];
  uint local_2c;
  undefined1 local_28 [20];
  undefined1 uStack_14;
  void *pvStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167dcd0;
  local_c = ExceptionList;
  iVar3 = param_1[5];
  if (iVar3 == 0x30000000) {
    if (param_1[3] == 0) {
      return (float10)0;
    }
    if (param_1[3] == 4) {
      return (float10)*(float *)param_1[4];
    }
    ExceptionList = &local_c;
    _00___FRawStaticIndexBuffer__vfunc_0();
  }
  else {
    if (iVar3 == 0x20000000) {
      ExceptionList = &local_c;
      iVar3 = (**(code **)(*param_1 + 0x48))();
      ExceptionList = pvStack_10;
      return (float10)iVar3;
    }
    ExceptionList = &local_c;
    if (iVar3 == 0x10000000) {
      if (0 < param_1[3]) {
        ExceptionList = &local_c;
        FUN_004384e0(local_44,(char *)param_1[4],param_1[3]);
        local_4 = 0;
        if (local_2c < 0x10) {
          local_40[0] = (char **)local_40;
        }
        dVar5 = atof((char *)local_40[0]);
        local_4 = 0xffffffff;
        ZipFileSystem__unknown_00433ed0((uint)local_44);
        ExceptionList = local_c;
        return (float10)(float)dVar5;
      }
      goto LAB_004748fe;
    }
  }
  _00___FRawStaticIndexBuffer__vfunc_0();
  PackedSection__unknown_00474320((int *)&local_48);
  local_4 = 1;
  piVar1 = (int *)FUN_00466c20((int)local_28);
  local_4 = CONCAT31(local_4._1_3_,2);
  uVar2 = (**(code **)(*param_1 + 0x58))(local_44);
  pvStack_10 = (void *)CONCAT31(pvStack_10._1_3_,3);
  (**(code **)(*local_48 + 0x8c))(uVar2);
  uStack_14 = 2;
  ZipFileSystem__unknown_00433ed0((uint)&stack0xffffffac);
  uStack_14 = 1;
  ZipFileSystem__unknown_00433ed0((uint)auStack_38);
  fVar4 = (float10)(**(code **)(*piVar1 + 0x50))(local_c);
  param_2 = (float)fVar4;
  local_4 = 0xffffffff;
  FUN_004585a0((int *)&local_48);
LAB_004748fe:
  ExceptionList = local_c;
  return (float10)param_2;
}


/* [VTable] PackedSection virtual function #24
   VTable: vtable_PackedSection at 017fdb9c */

undefined4 * __thiscall PackedSection__vfunc_24(void *this,undefined4 *param_1)

{
  undefined4 *puVar1;
  undefined4 uVar2;
  int *unaff_ESI;
  undefined4 unaff_EDI;
  int *local_48;
  undefined1 local_44 [4];
  undefined4 uStack_40;
  uint uStack_3c;
  undefined4 auStack_34 [3];
  undefined1 local_28 [4];
  void *pvStack_24;
  uint uStack_20;
  undefined4 uStack_1c;
  undefined1 uStack_14;
  undefined1 uStack_10;
  undefined4 *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167dcf8;
  local_c = ExceptionList;
  if ((*(int *)((int)this + 0x14) == 0x30000000) && (*(int *)((int)this + 0xc) == 8)) {
    puVar1 = *(undefined4 **)((int)this + 0x10);
    *param_1 = *puVar1;
    param_1[1] = puVar1[1];
    return param_1;
  }
  ExceptionList = &local_c;
  PackedSection__unknown_00474320((int *)&local_48);
  local_4 = 0;
  FUN_00466c20((int)local_28);
  local_4 = CONCAT31(local_4._1_3_,1);
  uVar2 = (**(code **)(*(int *)this + 0x58))(local_44);
  uStack_10 = 2;
  (**(code **)(*local_48 + 0x8c))(uVar2);
  uStack_14 = 1;
  if (0xf < uStack_3c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(unaff_EDI);
  }
  uStack_3c = 0xf;
  uStack_40 = 0;
  std::char_traits<char>::assign(&stack0xffffffb0,&stack0xffffffa7);
  uStack_14 = 0;
  if (0xf < uStack_20) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_34[0]);
  }
  uStack_20 = 0xf;
  pvStack_24 = (void *)0x0;
  std::char_traits<char>::assign((char *)auStack_34,&stack0xffffffa7);
  puVar1 = local_c;
  (**(code **)(*unaff_ESI + 0x60))(local_c,puStack_8);
  uStack_1c = 0xffffffff;
  FUN_004585a0((int *)&stack0xffffffa0);
  ExceptionList = pvStack_24;
  return puVar1;
}


/* [VTable] PackedSection virtual function #25
   VTable: vtable_PackedSection at 017fdb9c */

undefined8 * __thiscall PackedSection__vfunc_25(void *this,undefined8 *param_1)

{
  undefined8 *puVar1;
  undefined4 uVar2;
  int *unaff_ESI;
  undefined4 unaff_EDI;
  int *local_48;
  undefined1 local_44 [4];
  undefined4 uStack_40;
  uint uStack_3c;
  undefined4 auStack_34 [3];
  undefined1 local_28 [4];
  void *pvStack_24;
  uint uStack_20;
  undefined4 uStack_1c;
  undefined1 uStack_14;
  undefined1 uStack_10;
  undefined8 *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167dcf8;
  local_c = ExceptionList;
  if ((*(int *)((int)this + 0x14) == 0x30000000) && (*(int *)((int)this + 0xc) == 0xc)) {
    puVar1 = *(undefined8 **)((int)this + 0x10);
    *param_1 = *puVar1;
    *(undefined4 *)(param_1 + 1) = *(undefined4 *)(puVar1 + 1);
    return param_1;
  }
  ExceptionList = &local_c;
  PackedSection__unknown_00474320((int *)&local_48);
  local_4 = 0;
  FUN_00466c20((int)local_28);
  local_4 = CONCAT31(local_4._1_3_,1);
  uVar2 = (**(code **)(*(int *)this + 0x58))(local_44);
  uStack_10 = 2;
  (**(code **)(*local_48 + 0x8c))(uVar2);
  uStack_14 = 1;
  if (0xf < uStack_3c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(unaff_EDI);
  }
  uStack_3c = 0xf;
  uStack_40 = 0;
  std::char_traits<char>::assign(&stack0xffffffb0,&stack0xffffffa7);
  uStack_14 = 0;
  if (0xf < uStack_20) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_34[0]);
  }
  uStack_20 = 0xf;
  pvStack_24 = (void *)0x0;
  std::char_traits<char>::assign((char *)auStack_34,&stack0xffffffa7);
  puVar1 = local_c;
  (**(code **)(*unaff_ESI + 100))(local_c,puStack_8);
  uStack_1c = 0xffffffff;
  FUN_004585a0((int *)&stack0xffffffa0);
  ExceptionList = pvStack_24;
  return puVar1;
}


/* [VTable] PackedSection virtual function #26
   VTable: vtable_PackedSection at 017fdb9c */

undefined8 * __thiscall PackedSection__vfunc_26(void *this,undefined8 *param_1)

{
  undefined8 *puVar1;
  undefined4 uVar2;
  undefined4 unaff_ESI;
  int *unaff_EDI;
  int *local_48;
  undefined1 local_44 [4];
  undefined4 uStack_40;
  uint uStack_3c;
  undefined4 auStack_34 [3];
  undefined1 local_28 [4];
  void *pvStack_24;
  uint uStack_20;
  undefined4 uStack_1c;
  undefined1 uStack_14;
  undefined1 uStack_10;
  undefined8 *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167dcf8;
  local_c = ExceptionList;
  if ((*(int *)((int)this + 0x14) == 0x30000000) && (*(int *)((int)this + 0xc) == 0x10)) {
    puVar1 = *(undefined8 **)((int)this + 0x10);
    *param_1 = *puVar1;
    param_1[1] = puVar1[1];
    return param_1;
  }
  ExceptionList = &local_c;
  PackedSection__unknown_00474320((int *)&local_48);
  local_4 = 0;
  FUN_00466c20((int)local_28);
  local_4 = CONCAT31(local_4._1_3_,1);
  uVar2 = (**(code **)(*(int *)this + 0x58))(local_44);
  uStack_10 = 2;
  (**(code **)(*local_48 + 0x8c))(uVar2);
  uStack_14 = 1;
  if (0xf < uStack_3c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(unaff_ESI);
  }
  uStack_3c = 0xf;
  uStack_40 = 0;
  std::char_traits<char>::assign(&stack0xffffffb0,&stack0xffffffa7);
  uStack_14 = 0;
  if (0xf < uStack_20) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_34[0]);
  }
  uStack_20 = 0xf;
  pvStack_24 = (void *)0x0;
  std::char_traits<char>::assign((char *)auStack_34,&stack0xffffffa7);
  puVar1 = local_c;
  (**(code **)(*unaff_EDI + 0x68))(local_c,puStack_8);
  uStack_1c = 0xffffffff;
  FUN_004585a0((int *)&stack0xffffffa0);
  ExceptionList = pvStack_24;
  return puVar1;
}


/* [VTable] PackedSection virtual function #27
   VTable: vtable_PackedSection at 017fdb9c */

void __thiscall PackedSection__vfunc_27(void *this,undefined4 *param_1,undefined8 *param_2)

{
  undefined4 uVar1;
  uint uVar2;
  undefined8 *puVar3;
  int iVar4;
  undefined8 *puVar5;
  char local_91;
  int iStack_90;
  undefined4 local_8c [4];
  undefined4 local_7c;
  uint local_78;
  int iStack_74;
  undefined4 auStack_70 [4];
  undefined4 uStack_60;
  uint uStack_5c;
  undefined8 uStack_58;
  undefined8 local_4c;
  undefined4 local_44 [2];
  undefined8 uStack_3c;
  undefined4 uStack_34;
  undefined8 uStack_2c;
  undefined4 uStack_24;
  undefined8 uStack_1c;
  undefined4 uStack_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167dd36;
  local_c = ExceptionList;
  if ((*(int *)((int)this + 0x14) == 0x30000000) && (*(int *)((int)this + 0xc) == 0x30)) {
    ExceptionList = &local_c;
    memset(&local_4c,0,0x40);
    puVar5 = *(undefined8 **)((int)this + 0x10);
    local_10 = DAT_017f7ea0;
    puVar3 = &local_4c;
    iVar4 = 4;
    do {
      uVar1 = *(undefined4 *)(puVar5 + 1);
      *puVar3 = *puVar5;
      *(undefined4 *)(puVar3 + 1) = uVar1;
      puVar5 = (undefined8 *)((int)puVar5 + 0xc);
      puVar3 = puVar3 + 2;
      iVar4 = iVar4 + -1;
    } while (iVar4 != 0);
  }
  else {
    ExceptionList = &local_c;
    memset(&local_4c,0,0x40);
    local_10 = DAT_017f7ea0;
    local_78 = 0xf;
    local_91 = '\0';
    local_7c = 0;
    std::char_traits<char>::assign((char *)local_8c,&local_91);
    uVar2 = std::char_traits<char>::length("row0");
    FUN_004377d0(&iStack_90,"row0",uVar2);
    uStack_4 = 0;
    puVar3 = PackedSection__unknown_00468b50(this,&uStack_58,&iStack_90,param_2);
    local_44[0] = *(undefined4 *)(puVar3 + 1);
    local_4c = *puVar3;
    uStack_4 = 0xffffffff;
    if (0xf < local_78) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_8c[0]);
    }
    local_78 = 0xf;
    local_91 = '\0';
    local_7c = 0;
    std::char_traits<char>::assign((char *)local_8c,&local_91);
    uStack_5c = 0xf;
    local_91 = '\0';
    uStack_60 = 0;
    std::char_traits<char>::assign((char *)auStack_70,&local_91);
    uVar2 = std::char_traits<char>::length("row1");
    FUN_004377d0(&iStack_74,"row1",uVar2);
    uStack_4 = 1;
    puVar3 = PackedSection__unknown_00468b50(this,&uStack_58,&iStack_74,param_2 + 2);
    uStack_34 = *(undefined4 *)(puVar3 + 1);
    uStack_3c = *puVar3;
    uStack_4 = 0xffffffff;
    if (0xf < uStack_5c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_70[0]);
    }
    uStack_5c = 0xf;
    local_91 = '\0';
    uStack_60 = 0;
    std::char_traits<char>::assign((char *)auStack_70,&local_91);
    local_78 = 0xf;
    local_91 = '\0';
    local_7c = 0;
    std::char_traits<char>::assign((char *)local_8c,&local_91);
    uVar2 = std::char_traits<char>::length("row2");
    FUN_004377d0(&iStack_90,"row2",uVar2);
    uStack_4 = 2;
    puVar3 = PackedSection__unknown_00468b50(this,&uStack_58,&iStack_90,param_2 + 4);
    uStack_24 = *(undefined4 *)(puVar3 + 1);
    uStack_2c = *puVar3;
    uStack_4 = 0xffffffff;
    if (0xf < local_78) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_8c[0]);
    }
    local_78 = 0xf;
    local_91 = '\0';
    local_7c = 0;
    std::char_traits<char>::assign((char *)local_8c,&local_91);
    uStack_5c = 0xf;
    local_91 = '\0';
    uStack_60 = 0;
    std::char_traits<char>::assign((char *)auStack_70,&local_91);
    uVar2 = std::char_traits<char>::length("row3");
    FUN_004377d0(&iStack_74,"row3",uVar2);
    uStack_4 = 3;
    puVar3 = PackedSection__unknown_00468b50(this,&uStack_58,&iStack_74,param_2 + 6);
    uStack_14 = *(undefined4 *)(puVar3 + 1);
    uStack_1c = *puVar3;
    uStack_4 = 0xffffffff;
    if (0xf < uStack_5c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_70[0]);
    }
    uStack_5c = 0xf;
    local_91 = '\0';
    uStack_60 = 0;
    std::char_traits<char>::assign((char *)auStack_70,&local_91);
  }
  puVar3 = &local_4c;
  for (iVar4 = 0x10; iVar4 != 0; iVar4 = iVar4 + -1) {
    *param_1 = *(undefined4 *)puVar3;
    puVar3 = (undefined8 *)((int)puVar3 + 4);
    param_1 = param_1 + 1;
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] PackedSection virtual function #29
   VTable: vtable_PackedSection at 017fdb9c
   [String discovery] References: "PackedSection::asBlob" */

void * __thiscall PackedSection__vfunc_29(void *this,void *param_1,char param_2)

{
  uint uVar1;
  char *pcVar2;
  int iVar3;
  void *pvVar4;
  undefined4 uVar5;
  int *unaff_EBP;
  char local_6a;
  char cStack_69;
  undefined4 local_68;
  int *piStack_64;
  undefined1 auStack_60 [4];
  char local_5c [8];
  undefined1 auStack_54 [8];
  undefined4 local_4c;
  undefined4 local_48;
  undefined1 auStack_44 [4];
  undefined4 auStack_40 [2];
  undefined1 auStack_38 [8];
  undefined4 uStack_30;
  uint uStack_2c;
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = -1;
  puStack_8 = &LAB_0167dd89;
  local_c = ExceptionList;
  local_68 = 0;
  iVar3 = *(int *)((int)this + 0x14);
  if (iVar3 == 0x50000000) {
    uVar1 = *(uint *)((int)this + 0xc);
    pcVar2 = *(char **)((int)this + 0x10);
    ExceptionList = &local_c;
    *(undefined4 *)((int)param_1 + 0x18) = 0xf;
    param_2 = '\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<char>::assign((char *)((int)param_1 + 4),&param_2);
    FUN_004377d0(param_1,pcVar2,uVar1);
    ExceptionList = local_c;
    return param_1;
  }
  if ((iVar3 == 0x10000000) && (*(int *)((int)this + 0xc) == 0)) {
    ExceptionList = &local_c;
    BW__unknown_00438c40(param_1,"");
  }
  else {
    if ((iVar3 == 0x20000000) || (ExceptionList = &local_c, iVar3 == 0x40000000)) {
      local_48 = 0xf;
      local_6a = '\0';
      local_4c = 0;
      ExceptionList = &local_c;
      std::char_traits<char>::assign(local_5c,&local_6a);
      local_4 = 1;
      iVar3 = FUN_00466c20((int)auStack_28);
      local_4._0_1_ = 2;
      pvVar4 = (void *)(**(code **)(*(int *)this + 0x58))(auStack_44,iVar3,0);
      local_4._0_1_ = 3;
      uVar5 = FUN_00a35d30(pvVar4,auStack_60);
      cStack_69 = (char)uVar5;
      local_4._0_1_ = 2;
      if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
        scalable_free(auStack_40[0]);
      }
      uStack_2c = 0xf;
      local_6a = '\0';
      uStack_30 = 0;
      std::char_traits<char>::assign((char *)auStack_40,&local_6a);
      local_4._0_1_ = 1;
      if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
        scalable_free(auStack_24[0]);
      }
      uStack_10 = 0xf;
      local_6a = '\0';
      uStack_14 = 0;
      std::char_traits<char>::assign((char *)auStack_24,&local_6a);
      if (cStack_69 != '\0') {
        FUN_004384a0(param_1,auStack_60);
        local_68 = 1;
        local_4 = (uint)local_4._1_3_ << 8;
        ZipFileSystem__unknown_00433ed0((uint)auStack_60);
        ExceptionList = local_c;
        return param_1;
      }
      local_4 = (uint)local_4._1_3_ << 8;
      ZipFileSystem__unknown_00433ed0((uint)auStack_60);
    }
    _00___FRawStaticIndexBuffer__vfunc_0();
    PackedSection__unknown_00474320((int *)&piStack_64);
    local_4 = 4;
    iVar3 = FUN_00466c20((int)auStack_44);
    local_4._0_1_ = 5;
    uVar5 = (**(code **)(*(int *)this + 0x58))(auStack_28,iVar3,0);
    uStack_10 = CONCAT31(uStack_10._1_3_,6);
    (**(code **)(*piStack_64 + 0x8c))(uVar5);
    uStack_14._0_1_ = 5;
    ZipFileSystem__unknown_00433ed0((uint)auStack_38);
    uStack_14 = CONCAT31(uStack_14._1_3_,4);
    ZipFileSystem__unknown_00433ed0((uint)auStack_54);
    param_1 = local_c;
    (**(code **)(*unaff_EBP + 0x74))(local_c,puStack_8);
    local_68 = 1;
    local_4 = (uint)local_4._1_3_ << 8;
    FUN_004585a0((int *)&piStack_64);
  }
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] PackedSection virtual function #28
   VTable: vtable_PackedSection at 017fdb9c */

undefined4 * __thiscall PackedSection__vfunc_28(void *this,undefined4 *param_1)

{
  void *this_00;
  undefined4 *puVar1;
  uint uVar2;
  void *this_01;
  char local_61 [5];
  undefined1 *local_5c;
  undefined1 *local_58;
  undefined1 *puStack_54;
  exception local_50 [12];
  undefined1 auStack_44 [4];
  char local_40 [16];
  undefined4 local_30;
  uint local_2c;
  undefined1 auStack_28 [4];
  undefined4 **appuStack_24 [4];
  size_t sStack_14;
  uint uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167de07;
  local_c = ExceptionList;
  local_61[1] = '\0';
  local_61[2] = '\0';
  local_61[3] = '\0';
  local_61[4] = '\0';
  if (*(int *)((int)this + 0x14) == 0x50000000) {
    ExceptionList = &local_c;
    this_00 = (void *)scalable_malloc();
    if (this_00 == (void *)0x0) {
      FUN_004099f0(local_50);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_50,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_4 = 1;
    this_01 = *(void **)((int)this + 0x20);
    local_58 = &stack0xffffff88;
    local_5c = this_00;
    FUN_00473c40(this_01,(int *)&stack0xffffff88);
    local_4._1_3_ = (uint3)((uint)local_4 >> 8);
    local_4._0_1_ = 1;
    puVar1 = FUN_00472290(this_00,*(void **)((int)this + 0x10),*(size_t *)((int)this + 0xc),
                          (int)this_01);
    local_4 = (uint)local_4._1_3_ << 8;
    *param_1 = puVar1;
    if (puVar1 != (undefined4 *)0x0) {
      FUN_00457e40((int)puVar1);
    }
  }
  else {
    local_2c = 0xf;
    local_61[0] = '\0';
    local_30 = 0;
    ExceptionList = &local_c;
    std::char_traits<char>::assign(local_40,local_61);
    uVar2 = std::char_traits<char>::length("");
    FUN_004377d0(auStack_44,"",uVar2);
    local_4 = 4;
    (**(code **)(*(int *)this + 0x74))(auStack_28);
    local_4._0_1_ = 6;
    if (0xf < local_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free();
    }
    local_2c = 0xf;
    local_61[0] = '\0';
    local_30 = 0;
    std::char_traits<char>::assign(local_40,local_61);
    local_58 = (undefined1 *)scalable_malloc(0x14);
    if (local_58 == (void *)0x0) {
      FUN_004099f0(local_50);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_50,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_5c = &stack0xffffff88;
    puStack_54 = &stack0xffffff88;
    if (uStack_10 < 0x10) {
      appuStack_24[0] = appuStack_24;
    }
    local_4._0_1_ = 7;
    puVar1 = FUN_00472290(local_58,appuStack_24[0],sStack_14,0);
    local_4._0_1_ = 6;
    *param_1 = puVar1;
    if (puVar1 != (undefined4 *)0x0) {
      FUN_00457e40((int)puVar1);
    }
    local_61[1] = '\x01';
    local_61[2] = '\0';
    local_61[3] = '\0';
    local_61[4] = '\0';
    local_4 = (uint)local_4._1_3_ << 8;
    if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free();
    }
    uStack_10 = 0xf;
    local_61[0] = '\0';
    sStack_14 = 0;
    std::char_traits<char>::assign((char *)appuStack_24,local_61);
  }
  ExceptionList = local_c;
  return param_1;
}


/* WARNING: Variable defined which should be unmapped: param_2 */
/* [VTable] PackedSection virtual function #22
   VTable: vtable_PackedSection at 017fdb9c
   [String discovery] References: "PackedSection::asString" */

undefined1 * __thiscall PackedSection__vfunc_22(int *param_1,undefined1 *param_2)

{
  uint uVar1;
  int iVar2;
  char cVar3;
  undefined4 uVar4;
  int iVar5;
  int *unaff_ESI;
  float10 fVar6;
  char *pcVar7;
  int *local_120;
  char local_119 [5];
  undefined4 local_114;
  undefined4 local_110;
  undefined8 local_10c;
  undefined4 local_104;
  undefined1 local_100 [8];
  undefined1 local_f8 [28];
  undefined1 local_dc [12];
  undefined1 auStack_d0 [28];
  undefined1 local_b4 [16];
  undefined8 local_a4 [2];
  int local_94 [2];
  basic_ostream<char,struct_std::char_traits<char>_> abStack_8c [76];
  basic_ios<char,struct_std::char_traits<char>_> local_40 [48];
  undefined1 uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167de6b;
  local_c = ExceptionList;
  local_119[1] = '\0';
  local_119[2] = '\0';
  local_119[3] = '\0';
  local_119[4] = '\0';
  if (param_1[5] == 0x10000000) {
    uVar1 = param_1[3];
    pcVar7 = (char *)param_1[4];
    ExceptionList = &local_c;
    *(undefined4 *)(param_2 + 0x18) = 0xf;
    local_119[0] = '\0';
    *(undefined4 *)(param_2 + 0x14) = 0;
    std::char_traits<char>::assign(param_2 + 4,local_119);
    FUN_004377d0(param_2,pcVar7,uVar1);
  }
  else if (param_1[5] == 0x50000000) {
    ExceptionList = &local_c;
    FUN_00a35960(param_2,param_1[4],param_1[3]);
  }
  else {
    ExceptionList = &local_c;
    FUN_0046b6b0(local_94,3,1);
    local_4 = 1;
    iVar5 = param_1[5];
    if (iVar5 == 0x20000000) {
      iVar5 = (**(code **)(*param_1 + 0x48))(0);
      std::basic_ostream<char,struct_std::char_traits<char>_>::operator<<(abStack_8c,iVar5);
    }
    else if (iVar5 == 0x30000000) {
      if (param_1[3] != 0x30) {
        if ((param_1[3] & 3U) == 0) {
          PackedSection__unknown_00474320((int *)&local_120);
          local_4._0_1_ = 2;
          switch((uint)param_1[3] >> 2) {
          case 1:
            iVar5 = *local_120;
            fVar6 = (float10)(**(code **)(*param_1 + 0x50))(0,&DAT_017fd88c);
            (**(code **)(iVar5 + 0x84))((float)fVar6);
            break;
          case 2:
            local_114 = DAT_01ef06e4;
            local_110 = DAT_01ef06e8;
            iVar5 = *local_120;
            uVar4 = (**(code **)(*param_1 + 0x60))(local_100,&local_114,"%f %f");
            (**(code **)(iVar5 + 0x94))(uVar4);
            break;
          case 3:
            local_104 = DAT_01ef06f4;
            local_10c = DAT_01ef06ec;
            iVar5 = *local_120;
            uVar4 = (**(code **)(*param_1 + 100))(local_dc,&local_10c,"%f %f %f");
            (**(code **)(iVar5 + 0x98))(uVar4);
            break;
          case 4:
            iVar5 = *local_120;
            iVar2 = *param_1;
            pcVar7 = "%f %f %f %f";
            uVar4 = FUN_00466aa0(local_a4);
            uVar4 = (**(code **)(iVar2 + 0x68))(local_b4,uVar4,pcVar7);
            (**(code **)(iVar5 + 0x9c))(uVar4);
            break;
          default:
            _00___FRawStaticIndexBuffer__vfunc_0();
            BW__unknown_00438c40(local_f8,"");
            local_4._0_1_ = 3;
            (**(code **)(*local_120 + 0x8c))(local_f8);
            uStack_10 = 2;
            ZipFileSystem__unknown_00433ed0((uint)&local_104);
          }
          iVar5 = FUN_00466c20((int)local_dc);
          uStack_10 = 4;
          (**(code **)(*unaff_ESI + 0x58))(puStack_8,iVar5,0);
          local_119[1] = '\x01';
          local_119[2] = '\0';
          local_119[3] = '\0';
          local_119[4] = '\0';
          local_4._0_1_ = 2;
          ZipFileSystem__unknown_00433ed0((uint)auStack_d0);
          local_4._0_1_ = 1;
          FUN_004585a0((int *)&local_120);
          local_4 = (uint)local_4._1_3_ << 8;
          FUN_0046b690((int)local_94);
          ExceptionList = local_c;
          return param_2;
        }
        _00___FRawStaticIndexBuffer__vfunc_0();
      }
    }
    else if (iVar5 == 0x40000000) {
      cVar3 = (**(code **)(*param_1 + 0x44))(0);
      if (cVar3 == '\0') {
        ZipFileSystem__unknown_00426240(local_94,"false");
      }
      else {
        ZipFileSystem__unknown_00426240(local_94,"true");
      }
    }
    else {
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
    FUN_00475620(local_94,param_2);
    local_119[1] = '\x01';
    local_119[2] = '\0';
    local_119[3] = '\0';
    local_119[4] = '\0';
    local_4 = local_4 & 0xffffff00;
    FUN_0046b5f0((int)local_40);
    std::basic_ios<char,struct_std::char_traits<char>_>::
    ~basic_ios<char,struct_std::char_traits<char>_>(local_40);
  }
  ExceptionList = local_c;
  return param_2;
}


/* Also in vtable: PackedSection (slot 0) */

undefined4 * __thiscall PackedSection__vfunc_0(void *this,byte param_1)

{
  FUN_00473be0(this);
  if ((param_1 & 1) != 0) {
    FUN_00475c10(this);
  }
  return this;
}


/* [VTable] PackedSection virtual function #3
   VTable: vtable_PackedSection at 017fdb9c */

undefined4 * __thiscall PackedSection__vfunc_3(void *this,undefined4 *param_1,uint param_2)

{
  void *this_00;
  int *piVar1;
  int extraout_ECX;
  int iVar2;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167df54;
  local_c = ExceptionList;
  if (*(int *)((int)this + 0x1c) != 0) {
    ExceptionList = &local_c;
    PackedSection__unknown_00475c80
              ((void *)(*(int *)((int)this + 0x1c) + 2 + param_2 * 6),param_1,this);
    ExceptionList = local_c;
    return param_1;
  }
  if (((*(int *)((int)this + 0x14) == 0x30000000) && (*(int *)((int)this + 0xc) == 0x30)) &&
     (param_2 < 4)) {
    ExceptionList = &local_c;
    this_00 = (void *)FUN_00474430();
    local_4 = 1;
    if (this_00 == (void *)0x0) {
      piVar1 = (int *)0x0;
    }
    else {
      iVar2 = extraout_ECX;
      PackedSection__unknown_00473ca0(this,(int *)&stack0xffffffd8);
      local_4 = CONCAT31((int3)(local_4 >> 8),1);
      piVar1 = PackedSection_PackedSection
                         (this_00,(int)(&PTR_DAT_01dae348)[param_2],
                          *(int *)((int)this + 0x10) + param_2 * 0xc,0xc,0x30000000,iVar2);
    }
    local_4 = local_4 & 0xffffff00;
    FUN_0046bcb0(param_1,(int)piVar1,'\0');
    ExceptionList = local_c;
    return param_1;
  }
  *param_1 = 0;
  return param_1;
}


/* WARNING: Removing unreachable block (ram,0x004762be) */
/* [VTable] PackedSection virtual function #7
   VTable: vtable_PackedSection at 017fdb9c */

undefined4 * __thiscall PackedSection__vfunc_7(void *this,undefined4 *param_1,void *param_2)

{
  void *pvVar1;
  short sVar2;
  int iVar3;
  short sVar4;
  int iVar5;
  uint uVar6;
  char *pcVar7;
  char *pcVar8;
  int *piVar9;
  uint uVar10;
  int extraout_ECX;
  void *pvVar11;
  uint uVar12;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167df94;
  pvStack_c = ExceptionList;
  pvVar11 = (void *)0x0;
  ExceptionList = &pvStack_c;
  iVar5 = (**(code **)(*(int *)this + 4))();
  if (*(int *)((int)this + 0x1c) != 0) {
    pvVar11 = (void *)(*(int *)((int)this + 0x1c) + 2);
  }
  pvVar1 = (void *)((int)pvVar11 + iVar5 * 6);
  do {
    if (pvVar11 == pvVar1) {
      if ((((*(int *)((int)this + 0x14) == 0x30000000) && (*(int *)((int)this + 0xc) == 0x30)) &&
          ((*(int *)((int)param_2 + 0x14) == 4 &&
           ((pcVar8 = (char *)PackedSection__unknown_00457ec0(param_2,0), *pcVar8 == 'r' &&
            (pcVar8 = (char *)PackedSection__unknown_00457ec0(param_2,1), *pcVar8 == 'o')))))) &&
         (pcVar8 = (char *)PackedSection__unknown_00457ec0(param_2,2), *pcVar8 == 'w')) {
        pcVar8 = (char *)PackedSection__unknown_00457ec0(param_2,3);
        uVar12 = (int)*pcVar8 - 0x30;
        if (uVar12 < 4) {
          pvVar11 = (void *)FUN_00474430();
          uStack_4 = 1;
          if (pvVar11 == (void *)0x0) {
            piVar9 = (int *)0x0;
          }
          else {
            iVar5 = extraout_ECX;
            PackedSection__unknown_00473ca0(this,(int *)&stack0xffffffd4);
            uStack_4 = CONCAT31((int3)(uStack_4 >> 8),1);
            piVar9 = PackedSection_PackedSection
                               (pvVar11,(int)(&PTR_DAT_01dae358)[uVar12],
                                *(int *)((int)this + 0x10) + uVar12 * 0xc,0xc,0x30000000,iVar5);
          }
          uStack_4 = uStack_4 & 0xffffff00;
          FUN_0046bcb0(param_1,(int)piVar9,'\0');
          ExceptionList = pvStack_c;
          return param_1;
        }
      }
      *param_1 = 0;
      ExceptionList = pvStack_c;
      return param_1;
    }
    sVar2 = *(short *)((int)pvVar11 + 4);
    iVar5 = *(int *)((int)this + 0x20);
    if (sVar2 < 0) {
LAB_004762a5:
      pcVar8 = (char *)0x0;
    }
    else {
      iVar3 = *(int *)(iVar5 + 0x28);
      if (iVar3 == 0) {
        sVar4 = 0;
      }
      else {
        sVar4 = (short)(*(int *)(iVar5 + 0x2c) - iVar3 >> 2);
      }
      if (sVar4 <= sVar2) goto LAB_004762a5;
      if ((iVar3 == 0) || ((uint)(*(int *)(iVar5 + 0x2c) - iVar3 >> 2) <= (uint)(int)sVar2)) {
        _invalid_parameter_noinfo();
      }
      pcVar8 = *(char **)(*(int *)(iVar5 + 0x28) + sVar2 * 4);
    }
    uVar6 = std::char_traits<char>::length(pcVar8);
    uVar12 = *(uint *)((int)param_2 + 0x14);
    uVar10 = uVar12;
    if (uVar6 <= uVar12) {
      uVar10 = uVar6;
    }
    if (*(uint *)((int)param_2 + 0x18) < 0x10) {
      pcVar7 = (char *)((int)param_2 + 4);
    }
    else {
      pcVar7 = *(char **)((int)param_2 + 4);
    }
    iVar5 = std::char_traits<char>::compare(pcVar7,pcVar8,uVar10);
    if (((iVar5 == 0) && (uVar6 <= uVar12)) && (uVar12 == uVar6)) {
      PackedSection__unknown_00475c80(pvVar11,param_1,this);
      ExceptionList = pvStack_c;
      return param_1;
    }
    pvVar11 = (void *)((int)pvVar11 + 6);
  } while( true );
}




/* ========== PackedSectionFile.c ========== */

/*
 * SGW.exe - PackedSectionFile (1 functions)
 */

/* Also in vtable: PackedSectionFile (slot 0) */

undefined4 * __thiscall PackedSectionFile__vfunc_0(void *this,byte param_1)

{
  FUN_004741c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PacketEncrypter.c ========== */

/*
 * SGW.exe - PacketEncrypter (1 functions)
 */

undefined4 * __thiscall PacketEncrypter__vfunc_0(void *this,byte param_1)

{
  FUN_016039d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PidMessage.c ========== */

/*
 * SGW.exe - PidMessage (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage */

undefined4 * __thiscall PidMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_017942f8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = PidMessage::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this = MachineGuardMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== ProcessMessage.c ========== */

/*
 * SGW.exe - ProcessMessage (1 functions)
 */

undefined4 * __thiscall ProcessMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01577570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PythonDataType.c ========== */

/*
 * SGW.exe - PythonDataType (1 functions)
 */

undefined4 * __thiscall PythonDataType__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01795fa3;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_004585a0((int *)((int)this + 0x10));
  local_4 = 0xffffffff;
  FUN_01595b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== QueryInterfaceMessage.c ========== */

/*
 * SGW.exe - QueryInterfaceMessage (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage */

undefined4 * __thiscall QueryInterfaceMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_017942f8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = QueryInterfaceMessage::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this = MachineGuardMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== ResetMessage.c ========== */

/*
 * SGW.exe - ResetMessage (1 functions)
 */

/* References Mercury RTTI vtable: vtable_MachineGuardMessage */

undefined4 * __thiscall ResetMessage__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_017942f8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = MachineGuardMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== ServerConnection.c ========== */

/*
 * SGW.exe - ServerConnection (1 functions)
 */

undefined4 * __thiscall ServerConnection__vfunc_0(void *this,byte param_1)

{
  FUN_00dd8c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== SignalMessage.c ========== */

/*
 * SGW.exe - SignalMessage (1 functions)
 */

undefined4 * __thiscall SignalMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01794398;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = SignalMessage::vftable;
  local_4 = 0xffffffff;
  FUN_01577570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== TagsMessage.c ========== */

/*
 * SGW.exe - TagsMessage (1 functions)
 */

undefined4 * __thiscall TagsMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01587f60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TupleDataType.c ========== */

/*
 * SGW.exe - TupleDataType (1 functions)
 */

undefined4 * __thiscall TupleDataType__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01795d0b;
  local_c = ExceptionList;
  local_4 = 1;
  ExceptionList = &local_c;
  ServerModel__unknown_01189520((int *)((int)this + 0x10));
  local_4 = 0xffffffff;
  FUN_01595b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== UActorChannel.c ========== */

/*
 * SGW.exe - UActorChannel (10 functions)
 */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] UActorChannel virtual function #67
   VTable: vtable_UActorChannel at 018e8afc */

void __thiscall
UActorChannel__vfunc_67(void *this,int *param_1,undefined4 param_2,undefined4 param_3)

{
  undefined8 uVar1;
  int iVar2;
  undefined4 uVar3;
  
  iVar2 = (**(code **)(*param_1 + 0x130))();
  if (iVar2 == 0) {
    *(int **)((int)this + 0x3c) = param_1;
  }
  else {
    *(int *)((int)this + 0x3c) = param_1[0x13d6];
  }
  *(undefined4 *)((int)this + 0x44) = param_2;
  *(undefined4 *)((int)this + 0x48) = param_3;
  *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
  *(int *)((int)this + 0x5c) = param_1[0x34];
  uVar3 = (**(code **)(**(int **)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0x54) + 0xc))();
  *(undefined4 *)((int)this + 0x68) = uVar3;
  *(undefined8 *)((int)this + 0x74) =
       *(undefined8 *)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0x5c);
  uVar1 = _PTR_01801cf8;
  iVar2 = *(int *)(*(int *)((int)this + 0x3c) + 100);
  *(double *)((int)this + 0x7c) = *(double *)(iVar2 + 0x5c) - (double)*(float *)(iVar2 + 0x74);
  *(undefined8 *)((int)this + 0x84) = uVar1;
  *(uint *)((int)this + 0x8c) = *(uint *)((int)this + 0x8c) & 0xfffffff3 | 2;
  return;
}


/* Also in vtable: UActorChannel (slot 0) */

undefined4 UActorChannel__vfunc_0(void)

{
  return 1;
}


/* [VTable] UActorChannel virtual function #74
   VTable: vtable_UActorChannel at 018e8afc */

void __fastcall UActorChannel__vfunc_74(int *param_1)

{
  int *piVar1;
  bool bVar2;
  undefined3 extraout_var;
  
  (**(code **)(*param_1 + 0x110))();
  if (0 < param_1[0x25]) {
    FUN_0049f6e0(param_1[0x24],param_1[0x1c]);
  }
  if (*(int *)(*(int *)(param_1[0xf] + 100) + 0x50) == 0) {
    if ((param_1[0x1b] != 0) && ((*(byte *)(param_1 + 0x10) & 1) == 0)) {
      FUN_00554860((void *)(param_1[0xf] + 0x4ec4),param_1 + 0x1b);
    }
  }
  else {
    if (param_1[0x1b] != 0) {
      bVar2 = FUN_0049ef90(param_1[0x1b]);
      if (CONCAT31(extraout_var,bVar2) == 0) {
        FUN_00486000("Actor == NULL || Actor->IsValid()",".\\Src\\UnChan.cpp",0x313);
      }
    }
    piVar1 = (int *)param_1[0x1b];
    if (piVar1 != (int *)0x0) {
      if ((piVar1[0x1c] & 0x200000U) != 0) {
        *(undefined1 *)((int)piVar1 + 0x5a) = 3;
        *(undefined1 *)(param_1[0x1b] + 0x59) = 0;
        UChannel__vfunc_74((int)param_1);
        return;
      }
      if ((((piVar1[0x1c] & 0x4000U) == 0) && (DAT_01ee2684 != (void *)0x0)) && (DAT_01ead7ec == 0))
      {
        FUN_00875290(DAT_01ee2684,piVar1,1,1);
        UChannel__vfunc_74((int)param_1);
        return;
      }
    }
  }
  UChannel__vfunc_74((int)param_1);
  return;
}


/* [VTable] UActorChannel virtual function #1
   VTable: vtable_UActorChannel at 018e8afc */

bool __fastcall UActorChannel__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee64f0 == (undefined4 *)0x0) {
    DAT_01ee64f0 = FUN_008c7980();
    FUN_008c7a50();
  }
  return puVar1 != DAT_01ee64f0;
}


/* [VTable] UActorChannel virtual function #68
   VTable: vtable_UActorChannel at 018e8afc */

void __fastcall UActorChannel__vfunc_68(int param_1)

{
  if (*(int *)(param_1 + 0x6c) != 0) {
    UActorChannel__unknown_010ba320
              ((void *)(*(int *)(param_1 + 0x3c) + 0x4ed0),*(int *)(param_1 + 0x6c));
  }
  *(uint *)(param_1 + 0x40) = *(uint *)(param_1 + 0x40) | 2;
  return;
}


/* [VTable] UActorChannel virtual function #72
   VTable: vtable_UActorChannel at 018e8afc */

void __thiscall UActorChannel__vfunc_72(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  iVar2 = param_1;
  UChannel__vfunc_72(this,param_1);
  if (*(int *)((int)this + 0x70) == 0) {
    *(uint *)((int)this + 0x8c) = *(uint *)((int)this + 0x8c) | 2;
    return;
  }
  iVar1 = *(int *)((int)this + 0xb8);
  while (iVar1 = iVar1 + -1, -1 < iVar1) {
    if ((*(int *)(*(int *)((int)this + 0xb4) + 4 + iVar1 * 0xc) == iVar2) &&
       (*(char *)(*(int *)((int)this + 0xb4) + iVar1 * 0xc + 8) == '\0')) {
      param_1 = iVar1;
      FUN_005602a0((void *)((int)this + 0xa8),&param_1);
    }
  }
  *(uint *)((int)this + 0x8c) = *(uint *)((int)this + 0x8c) | 2;
  return;
}


/* [VTable] UActorChannel virtual function #70
   VTable: vtable_UActorChannel at 018e8afc */

void * __thiscall UActorChannel__vfunc_70(void *this,void *param_1)

{
  undefined4 *puVar1;
  void *this_00;
  undefined4 *this_01;
  int local_30 [3];
  int local_24 [3];
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdbc4;
  local_c = ExceptionList;
  if (((*(byte *)((int)this + 0x40) & 2) == 0) && (*(int *)((int)this + 0x6c) != 0)) {
    ExceptionList = &local_c;
    puVar1 = UChannel__vfunc_70(this,local_18);
    local_4 = 3;
    FUN_004a0be0(*(void **)((int)this + 0x6c),local_24,(void *)0x0);
    local_4._0_1_ = 4;
    this_00 = FUN_008cbcc0(local_30);
    local_4._0_1_ = 5;
    FUN_0041b600(this_00,param_1,puVar1);
    local_4._0_1_ = 4;
    FUN_0041b420(local_30);
    local_4._0_1_ = 3;
    FUN_0041b420(local_24);
    local_4 = (uint)local_4._1_3_ << 8;
    FUN_0041b420(local_18);
    ExceptionList = local_c;
    return param_1;
  }
  ExceptionList = &local_c;
  puVar1 = UChannel__vfunc_70(this,local_24);
  local_4 = 1;
  this_01 = FUN_0041aab0(local_18,L"Actor=None ");
  local_4._0_1_ = 2;
  FUN_0041b600(this_01,param_1,puVar1);
  local_4._0_1_ = 1;
  FUN_0041b420(local_18);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_0041b420(local_24);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] UActorChannel virtual function #71
   VTable: vtable_UActorChannel at 018e8afc */

void __thiscall UActorChannel__vfunc_71(void *this,int *param_1)

{
  uint *puVar1;
  int *piVar2;
  size_t _Size;
  bool bVar3;
  bool bVar4;
  undefined1 uVar5;
  int *piVar6;
  int iVar7;
  undefined4 *puVar8;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  undefined1 *puVar9;
  int iVar10;
  uint uVar11;
  undefined3 extraout_var_01;
  int *piVar12;
  void *pvVar13;
  bool bVar14;
  int *piStack_78;
  int iStack_68;
  int local_64;
  int iStack_5c;
  int *piStack_58;
  undefined4 uStack_54;
  undefined4 uStack_50;
  undefined1 auStack_4c [8];
  undefined1 auStack_44 [8];
  int iStack_3c;
  int iStack_38;
  int iStack_34;
  uint uStack_2c;
  int iStack_28;
  float fStack_24;
  uint uStack_20;
  int iStack_1c;
  void *local_14;
  undefined1 *puStack_10;
  undefined4 uStack_c;
  
  uStack_c = 0xffffffff;
  puStack_10 = &LAB_016bdd39;
  local_14 = ExceptionList;
  ExceptionList = &local_14;
  if ((*(byte *)((int)this + 0x40) & 2) != 0) {
    ExceptionList = &local_14;
    FUN_00486000("!Closing",".\\Src\\UnChan.cpp",0x394);
  }
  if ((*(byte *)((int)this + 0x40) & 0x18) != 0) {
    ExceptionList = local_14;
    return;
  }
  bVar3 = false;
  if (*(int *)((int)this + 0x6c) == 0) {
    if ((char)param_1[0x26] == '\0') {
      ExceptionList = local_14;
      return;
    }
    (**(code **)(*param_1 + 0x18))();
    piVar6 = (int *)FUN_005d0910(local_64);
    if (piVar6 == (int *)0x0) {
      *(uint *)((int)this + 0x40) = *(uint *)((int)this + 0x40) | 8;
      ExceptionList = local_14;
      return;
    }
    if ((piVar6[2] & 0x600U) != 0) {
      iStack_3c = 0;
      iStack_38 = 0;
      iStack_34 = 0;
      uVar11 = piVar6[0x1c];
      FUN_008c7760(&fStack_24);
      if ((uVar11 >> 0x17 & 1) != 0) {
        FUN_008c7660();
      }
      piVar6 = FUN_00876970(DAT_01ee2684,(int *)piVar6[0xd],0,0,(undefined8 *)&fStack_24,
                            (undefined8 *)&iStack_3c,piVar6,1,1,(void *)0x0,0,1);
      bVar3 = true;
      if (piVar6 == (int *)0x0) {
        FUN_00486000("InActor",".\\Src\\UnChan.cpp",0x3bf);
      }
    }
    FUN_008c95c0(this,(int)piVar6);
    iVar7 = *(int *)(*(int *)((int)this + 0x3c) + 100);
    if (((iVar7 != 0) && (*(int *)((int)this + 0x3c) == *(int *)(iVar7 + 0x50))) &&
       (iVar7 = (**(code **)(**(int **)((int)this + 0x6c) + 0x2a4))(), iVar7 != 0)) {
      piVar6 = *(int **)((int)this + 0x3c);
      if (piVar6[0x1a] != 2) {
        if ((piVar6[0x1a] != 3) || (*(int *)((int)this + 0x6c) == piVar6[0x10])) goto LAB_008c9bac;
        piVar6 = (int *)(**(code **)(*(int *)piVar6[0x19] + 0x134))();
      }
      (**(code **)(*piVar6 + 0x124))();
    }
  }
LAB_008c9bac:
  piVar6 = FUN_004ff450(*(void **)(*(int *)((int)this + 0x3c) + 0x60),*(int **)((int)this + 0x70));
  puVar1 = (uint *)(*(int *)((int)this + 0x6c) + 0x74);
  *puVar1 = *puVar1 & 0xfeffffff;
  iVar7 = (**(code **)(**(int **)((int)this + 0x6c) + 700))();
  if (iVar7 == 0) goto LAB_008c9c33;
  pvVar13 = *(void **)(iVar7 + 0x2ac);
  if (*(int *)(*(int *)((int)*(void **)((int)this + 0x3c) + 100) + 0x50) == 0) {
    if (pvVar13 != *(void **)((int)this + 0x3c)) {
      if (pvVar13 == (void *)0x0) goto LAB_008c9c33;
      puVar8 = FUN_00533580();
      bVar4 = FUN_00419370(pvVar13,(int)puVar8);
      if ((CONCAT31(extraout_var_00,bVar4) == 0) ||
         (*(int *)((int)pvVar13 + 0x4f58) != *(int *)((int)this + 0x3c))) goto LAB_008c9c33;
    }
  }
  else {
    if (pvVar13 == (void *)0x0) goto LAB_008c9c33;
    puVar8 = FUN_00534470();
    bVar4 = FUN_00419370(pvVar13,(int)puVar8);
    if (CONCAT31(extraout_var,bVar4) == 0) goto LAB_008c9c33;
  }
  puVar1 = (uint *)(*(int *)((int)this + 0x6c) + 0x74);
  *puVar1 = *puVar1 | 0x1000000;
LAB_008c9c33:
  puVar9 = UActorChannel__unknown_0156ba70(param_1,piVar6[3] + piVar6[8]);
  if (param_1[0xb] == 0) {
    piStack_78 = (int *)FUN_008cb9a0(piVar6,(int)puVar9);
  }
  else {
    piStack_78 = (int *)0x0;
  }
  bVar4 = *(int *)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0x50) == 0;
LAB_008c9c74:
  if ((piStack_78 == (int *)0x0) && (!bVar3)) {
    if (*(int *)((int)this + 0x6c) == 0) {
      ExceptionList = local_14;
      return;
    }
    if ((*(uint *)(*(int *)((int)this + 0x6c) + 0x70) & 0x200000) == 0) {
      ExceptionList = local_14;
      return;
    }
    uVar5 = FUN_0054d8c0((int)DAT_01ee2684);
    if (CONCAT31(extraout_var_01,uVar5) != 3) {
      ExceptionList = local_14;
      return;
    }
    *(undefined1 *)(*(int *)((int)this + 0x6c) + 0x5a) = uVar5;
    *(undefined1 *)(*(int *)((int)this + 0x6c) + 0x59) = 0;
    *(uint *)((int)this + 0x40) = *(uint *)((int)this + 0x40) | 0x10;
    UActorChannel__unknown_010ba320
              ((void *)(*(int *)((int)this + 0x3c) + 0x4ed0),*(int *)((int)this + 0x6c));
    FUN_008cb890(*(int **)((int)this + 0x6c));
    *(undefined4 *)((int)this + 0x6c) = 0;
    ExceptionList = local_14;
    return;
  }
  bVar14 = piStack_78 != (int *)0x0;
  if ((bVar14) && (!bVar4)) {
    (**(code **)(**(int **)((int)this + 0x6c) + 0x170))();
  }
  iStack_3c = 0;
  iStack_38 = 0;
  iStack_34 = 0;
  uStack_c = 1;
joined_r0x008c9cba:
  do {
    if (((piStack_78 == (int *)0x0) || (piVar12 = (int *)*piStack_78, piVar12 == (int *)0x0)) ||
       ((*(uint *)(piVar12[0xd] + 0xb8) & 0x8000) == 0)) break;
    if (piVar12[0x11] != 1) {
      (**(code **)(*param_1 + 4))();
    }
    iVar7 = *(int *)((int)this + 0x6c);
    if (*(int *)((int)this + 0x94) == 0) {
      piStack_78 = (int *)0x0;
    }
    else {
      piStack_78 = *(int **)((int)this + 0x90);
    }
    if (bVar4) {
      iVar7 = 0;
      piStack_78 = (int *)0x0;
    }
    piVar2 = (int *)(*(int *)((int)this + 0xb4) + (uint)*(ushort *)((int)piVar12 + 0x5e) * 0xc);
    if (param_1[0x20] < *piVar2) {
      iVar7 = 0;
      piStack_78 = (int *)0x0;
    }
    else {
      *piVar2 = param_1[0x20];
    }
    _Size = piVar12[0x12];
    uStack_20 = DAT_01ea5750;
    iStack_1c = DAT_01ea575c;
    if (iVar7 == 0) {
      pvVar13 = (void *)(DAT_01ea5750 + 7 & 0xfffffff8);
      DAT_01ea5750 = (int)pvVar13 + _Size;
      if (DAT_01ea5754 < DAT_01ea5750) {
        UEditorEngine__unknown_004fc410(&DAT_01ea5750,_Size + 8);
        pvVar13 = (void *)(DAT_01ea5750 + 7 & 0xfffffff8);
        DAT_01ea5750 = (int)pvVar13 + _Size;
      }
      memset(pvVar13,0,_Size);
    }
    (**(code **)(*piVar12 + 0x134))();
    if (piStack_78 != (int *)0x0) {
      (**(code **)(*piVar12 + 0x148))();
    }
    if (iStack_1c != DAT_01ea575c) {
      FUN_004fc360(&DAT_01ea5750,iStack_1c);
    }
    iVar7 = iStack_38;
    DAT_01ea5750 = uStack_20;
    if ((piVar12[0x14] & 1U) != 0) {
      iStack_38 = iStack_38 + 1;
      if ((iStack_34 < iStack_38) &&
         ((iStack_34 = ((int)(iStack_38 * 3 + (iStack_38 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iStack_38
          , iStack_3c != 0 || (iStack_34 != 0)))) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        iStack_3c = (**(code **)(*DAT_01ea5778 + 8))();
      }
      piVar2 = (int *)(iStack_3c + iVar7 * 4);
      if (piVar2 != (int *)0x0) {
        *piVar2 = (int)piVar12;
      }
    }
    puVar9 = UActorChannel__unknown_0156ba70(param_1,piVar6[3] + piVar6[8]);
    piVar12 = piVar6;
    if (param_1[0xb] == 0) {
      do {
        iVar7 = piVar12[3];
        if ((iVar7 <= (int)puVar9) && ((int)puVar9 < piVar12[8] + iVar7)) {
          piStack_78 = (int *)(piVar12[7] + ((int)puVar9 - iVar7) * 0xc);
          goto joined_r0x008c9cba;
        }
        piVar2 = piVar12 + 4;
        piVar12 = (int *)*piVar2;
      } while ((int *)*piVar2 != (int *)0x0);
    }
    piStack_78 = (int *)0x0;
  } while( true );
  if (!bVar4) {
    if (bVar14) {
      (**(code **)(**(int **)((int)this + 0x6c) + 0x174))();
      if (*(int *)((int)this + 0x6c) == 0) goto LAB_008ca27c;
      iStack_68 = 0;
      if (0 < iStack_38) {
        do {
          iVar7 = *(int *)(iStack_3c + iStack_68 * 4);
          if (*(int *)(iVar7 + 4) == -1) {
            puVar8 = FUN_0049e960(auStack_4c,L"<uninitialized>",1);
          }
          else {
            puVar8 = (undefined4 *)(iVar7 + 0x2c);
          }
          uStack_54 = *puVar8;
          uStack_50 = puVar8[1];
          iVar7 = **(int **)((int)this + 0x6c);
          FUN_0049ffb0(*(int **)((int)this + 0x6c),DAT_01ee179c,DAT_01ee17a0,0);
          (**(code **)(iVar7 + 0xe8))();
          if (*(int *)((int)this + 0x6c) == 0) goto LAB_008ca27c;
          iStack_68 = iStack_68 + 1;
        } while (iStack_68 < iStack_38);
      }
    }
    bVar3 = false;
  }
  if (piStack_78 != (int *)0x0) {
    iVar7 = *piStack_78;
    if ((iVar7 == 0) || ((*(uint *)(*(int *)(iVar7 + 0x34) + 0xb8) & 0x80000) == 0)) {
LAB_008ca27c:
      uStack_c = 0xffffffff;
      FUN_004b7a90(&iStack_3c);
      ExceptionList = local_14;
      return;
    }
    if (*(int *)(iVar7 + 4) == -1) {
      puVar8 = FUN_0049e960(auStack_44,L"<uninitialized>",1);
    }
    else {
      puVar8 = (undefined4 *)(iVar7 + 0x2c);
    }
    iVar7 = FUN_0049ff40(*(void **)((int)this + 0x6c),*puVar8,puVar8[1],0);
    if (iVar7 == 0) {
      FUN_00486000("Function",".\\Src\\UnChan.cpp",0x45a);
    }
    uStack_2c = DAT_01ea5750;
    iStack_28 = DAT_01ea575c;
    FUN_008cb830(1,&DAT_01ea5750,1,(uint)*(ushort *)(iVar7 + 0x9e),8);
    piStack_58 = *(int **)(iVar7 + 0x4c);
    iStack_5c = iVar7;
    FUN_004ad990(&iStack_5c);
    piVar12 = piStack_58;
    while ((piStack_58 = piVar12, piVar12 != (int *)0x0 && ((piVar12[0x13] & 0x480U) == 0x80))) {
      iVar10 = (**(code **)(**(int **)(*(int *)((int)this + 0x3c) + 0x60) + 0x114))();
      if ((iVar10 != 0) &&
         ((((*(uint *)(piVar12[0xd] + 0xb8) & 0x20000) != 0 ||
           (uVar11 = UActorChannel__unknown_0156baa0(param_1), (char)uVar11 != '\0')) &&
          (iVar10 = 0, 0 < piVar12[0x11])))) {
        do {
          (**(code **)(*piVar12 + 0x134))();
          iVar10 = iVar10 + 1;
        } while (iVar10 < piVar12[0x11]);
      }
      piStack_58 = (int *)piVar12[0x10];
      FUN_004ad990(&iStack_5c);
      piVar12 = piStack_58;
    }
    (**(code **)(**(int **)((int)this + 0x6c) + 0xe8))();
    for (piVar12 = *(int **)(iVar7 + 0x78); piVar12 != (int *)0x0; piVar12 = (int *)piVar12[0x1c]) {
      if (piVar12[0x19] < (int)(uint)*(ushort *)(iVar7 + 0x9e)) {
        (**(code **)(*piVar12 + 0x158))();
      }
    }
    if (iStack_28 != DAT_01ea575c) {
      FUN_004fc360(&DAT_01ea5750,iStack_28);
    }
    DAT_01ea5750 = uStack_2c;
    if (*(int *)((int)this + 0x6c) == 0) goto LAB_008ca27c;
    puVar9 = UActorChannel__unknown_0156ba70(param_1,piVar6[3] + piVar6[8]);
    piVar12 = piVar6;
    if (param_1[0xb] == 0) {
      do {
        iVar7 = piVar12[3];
        if ((iVar7 <= (int)puVar9) && ((int)puVar9 < piVar12[8] + iVar7)) {
          piStack_78 = (int *)(piVar12[7] + ((int)puVar9 - iVar7) * 0xc);
          goto LAB_008ca1f1;
        }
        piVar2 = piVar12 + 4;
        piVar12 = (int *)*piVar2;
      } while ((int *)*piVar2 != (int *)0x0);
    }
    piStack_78 = (int *)0x0;
  }
LAB_008ca1f1:
  uStack_c = 0xffffffff;
  FUN_004b7a90(&iStack_3c);
  goto LAB_008c9c74;
}


/* [VTable] UActorChannel virtual function #69
   VTable: vtable_UActorChannel at 018e8afc */

void __fastcall UActorChannel__vfunc_69(void *param_1)

{
  int iVar1;
  int iVar2;
  int *piVar3;
  int iVar4;
  int *local_1c;
  int *local_18;
  int local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bde73;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  UChannel__vfunc_69(param_1);
  if (*(int *)((int)param_1 + 0x6c) != 0) {
    UActorChannel__unknown_010ba320
              ((void *)(*(int *)((int)param_1 + 0x3c) + 0x4ed0),*(int *)((int)param_1 + 0x6c));
    if (((*(byte *)(*(int *)((int)param_1 + 0x6c) + 0x70) & 5) == 0) &&
       ((*(byte *)((int)param_1 + 0x8c) & 0x20) != 0)) {
      iVar1 = *(int *)((int)param_1 + 0x3c);
      local_1c = (int *)(iVar1 + 0x4ed0);
      local_10 = 0;
      for (local_14 = 0; (-1 < local_14 && (local_14 < *(int *)(iVar1 + 0x4ed4)));
          local_14 = local_14 + 1) {
        iVar2 = *(int *)(*local_1c + 8 + local_14 * 0xc);
        if ((((iVar2 != 0) && (*(int *)(iVar2 + 0x6c) != 0)) && ((*(byte *)(iVar2 + 0x40) & 2) == 0)
            ) && (iVar4 = 0, 0 < *(int *)(iVar2 + 0xc4))) {
          do {
            piVar3 = (int *)(*(int *)(*(int *)(*(int *)(iVar2 + 0xc0) + iVar4 * 4) + 100) +
                            *(int *)(iVar2 + 0x90));
            if (*piVar3 == *(int *)((int)param_1 + 0x6c)) {
              *piVar3 = 0;
              *(uint *)(iVar2 + 0x8c) = *(uint *)(iVar2 + 0x8c) | 2;
            }
            iVar4 = iVar4 + 1;
          } while (iVar4 < *(int *)(iVar2 + 0xc4));
        }
      }
      local_4 = 0xffffffff;
      local_18 = local_1c;
      FUN_00865300((int *)&local_1c);
    }
    *(undefined4 *)((int)param_1 + 0x6c) = 0;
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] UActorChannel virtual function #2
   VTable: vtable_UActorChannel at 018e8afc */

int * __thiscall UActorChannel__vfunc_2(void *this,byte param_1)

{
  FUN_008cb9e0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UBWAvatarFilter.c ========== */

/*
 * SGW.exe - UBWAvatarFilter (1 functions)
 */

undefined4 UBWAvatarFilter__vfunc_0(void)

{
  return 1;
}




/* ========== UBWConnection.c ========== */

/*
 * SGW.exe - UBWConnection (3 functions)
 */

/* Also in vtable: UBWConnection (slot 0) */

undefined4 UBWConnection__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBWConnection virtual function #1
   VTable: vtable_UBWConnection at 0180167c */

bool __fastcall UBWConnection__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2bbc == (undefined4 *)0x0) {
    DAT_01ee2bbc = FUN_005e0a10();
    FUN_005de2b0();
  }
  return puVar1 != DAT_01ee2bbc;
}


/* [VTable] UBWConnection virtual function #2
   VTable: vtable_UBWConnection at 0180167c */

int * __thiscall UBWConnection__vfunc_2(void *this,byte param_1)

{
  FUN_00480b30(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UBWExportCommandlet.c ========== */

/*
 * SGW.exe - UBWExportCommandlet (1 functions)
 */

undefined4 UBWExportCommandlet__vfunc_0(void)

{
  return 1;
}




/* ========== UBWFilter.c ========== */

/*
 * SGW.exe - UBWFilter (1 functions)
 */

undefined4 UBWFilter__vfunc_0(void)

{
  return 1;
}




/* ========== UBWNetDriver.c ========== */

/*
 * SGW.exe - UBWNetDriver (3 functions)
 */

/* Also in vtable: UBWNetDriver (slot 0) */

undefined4 UBWNetDriver__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBWNetDriver virtual function #1
   VTable: vtable_UBWNetDriver at 018014f4 */

bool __fastcall UBWNetDriver__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2bb8 == (undefined4 *)0x0) {
    DAT_01ee2bb8 = FUN_005dd860();
    FUN_005dc080();
  }
  return puVar1 != DAT_01ee2bb8;
}


/* [VTable] UBWNetDriver virtual function #2
   VTable: vtable_UBWNetDriver at 018014f4 */

int * __thiscall UBWNetDriver__vfunc_2(void *this,byte param_1)

{
  FUN_004807d0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UBWNullFilter.c ========== */

/*
 * SGW.exe - UBWNullFilter (1 functions)
 */

undefined4 UBWNullFilter__vfunc_0(void)

{
  return 1;
}




/* ========== UBigWorldInfo.c ========== */

/*
 * SGW.exe - UBigWorldInfo (4 functions)
 */

/* [VTable] UBigWorldInfo virtual function #67
   VTable: vtable_UBigWorldInfo at 018caea4 */

void UBigWorldInfo__vfunc_67(void)

{
  return;
}


/* Also in vtable: UBigWorldInfo (slot 0) */

undefined4 UBigWorldInfo__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBigWorldInfo virtual function #1
   VTable: vtable_UBigWorldInfo at 018caea4 */

bool __fastcall UBigWorldInfo__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UBigWorldInfo virtual function #2
   VTable: vtable_UBigWorldInfo at 018caea4 */

int * __thiscall UBigWorldInfo__vfunc_2(void *this,byte param_1)

{
  FUN_0084b160(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UChannel.c ========== */

/*
 * SGW.exe - UChannel (5 functions)
 */

/* [VTable] UChannel virtual function #2
   VTable: vtable_UChannel at 018e889c */

int * __thiscall UChannel__vfunc_2(void *this,byte param_1)

{
  FUN_008c71f0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UChannel virtual function #67
   VTable: vtable_UChannel at 018e889c */

void __thiscall UChannel__vfunc_67(void *this,int *param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  
  iVar1 = (**(code **)(*param_1 + 0x130))();
  if (iVar1 == 0) {
    *(int **)((int)this + 0x3c) = param_1;
  }
  else {
    *(int *)((int)this + 0x3c) = param_1[0x13d6];
  }
  *(undefined4 *)((int)this + 0x44) = param_2;
  *(undefined4 *)((int)this + 0x48) = param_3;
  *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
  *(int *)((int)this + 0x5c) = param_1[0x34];
  return;
}


/* Also in vtable: UChannel (slot 0) */

undefined4 UChannel__vfunc_0(void)

{
  return 1;
}


/* [VTable] UChannel virtual function #1
   VTable: vtable_UChannel at 018e889c */

bool __fastcall UChannel__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UChannel virtual function #70
   VTable: vtable_UChannel at 018e889c */

void * __thiscall UChannel__vfunc_70(void *this,void *param_1)

{
  undefined4 *this_00;
  wchar_t *pwVar1;
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdaeb;
  local_c = ExceptionList;
  pwVar1 = L"closing";
  if ((*(byte *)((int)this + 0x40) & 2) == 0) {
    pwVar1 = L"open";
  }
  ExceptionList = &local_c;
  this_00 = FUN_0041aab0(local_18,L"State=");
  local_4 = 1;
  FUN_0041b580(this_00,param_1,pwVar1);
  local_4 = local_4 & 0xffffff00;
  FUN_0041b420(local_18);
  ExceptionList = local_c;
  return param_1;
}




/* ========== UChannelDownload.c ========== */

/*
 * SGW.exe - UChannelDownload (7 functions)
 */

/* [VTable] UChannelDownload virtual function #12
   VTable: vtable_UChannelDownload at 018e7dac */

void __thiscall UChannelDownload__vfunc_12(void *this,int *param_1)

{
  UTestIpDrv__vfunc_12(this,param_1);
  (**(code **)(*param_1 + 0x18))((int)this + 0x3c);
  (**(code **)(*param_1 + 0x18))((int)this + 0x468);
  return;
}


/* [VTable] UChannelDownload virtual function #11
   VTable: vtable_UChannelDownload at 018e7dac */

void __fastcall UChannelDownload__vfunc_11(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 0x468);
  if ((iVar1 != 0) && (*(int *)(iVar1 + 0x68) == param_1)) {
    *(undefined4 *)(iVar1 + 0x68) = 0;
  }
  *(undefined4 *)(param_1 + 0x468) = 0;
  UDownload__vfunc_11(param_1);
  return;
}


/* Also in vtable: UChannelDownload (slot 0) */

undefined4 UChannelDownload__vfunc_0(void)

{
  return 1;
}


/* [VTable] UChannelDownload virtual function #67
   VTable: vtable_UChannelDownload at 018e7dac */

undefined4 __fastcall UChannelDownload__vfunc_67(int param_1)

{
  void *this;
  int local_c0 [3];
  int local_b4 [41];
  undefined1 local_e;
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bd6c6;
  local_c = ExceptionList;
  if (((*(int *)(param_1 + 0x468) != 0) && (*(int *)(param_1 + 0x58) != 0)) &&
     ((*(byte *)(*(int *)(param_1 + 0x44) + 0x30) & 2) != 0)) {
    ExceptionList = &local_c;
    *(undefined4 *)(param_1 + 0x460) = 1;
    FUN_00957550(local_b4,*(int *)(param_1 + 0x468),1);
    local_4 = 0;
    FUN_0041aab0(local_c0,L"SKIP");
    local_4._0_1_ = 1;
    FUN_00485df0(this,local_b4,local_c0);
    local_e = 1;
    FUN_008c8e80(*(void **)(param_1 + 0x468),local_b4,0);
    local_4 = (uint)local_4._1_3_ << 8;
    FUN_0041b420(local_c0);
    local_4 = 0xffffffff;
    FUN_005e1190(local_b4);
    ExceptionList = local_c;
    return 1;
  }
  return 0;
}


/* [VTable] UChannelDownload virtual function #68
   VTable: vtable_UChannelDownload at 018e7dac */

void __thiscall UChannelDownload__vfunc_68(void *this,void *param_1,int param_2)

{
  int iVar1;
  int *piVar2;
  undefined4 *puVar3;
  undefined **ppuVar4;
  undefined1 local_c0 [12];
  undefined4 local_b4 [11];
  int local_88;
  undefined1 local_e;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bd6e6;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(void **)((int)this + 0x3c) = param_1;
  *(int *)((int)this + 0x40) = param_2;
  *(int *)((int)this + 0x44) = *(int *)(*(int *)((int)param_1 + 0x60) + 0x3c) + param_2 * 0x3c;
  piVar2 = Mercury_Channel_7(param_1,3,1,0xffffffff);
  *(int **)((int)this + 0x468) = piVar2;
  if (piVar2 == (int *)0x0) {
    puVar3 = FUN_00489340(local_c0,L"ChAllocate",L"Engine",(wchar_t *)0x0);
    local_4 = 0;
    if (puVar3[1] == 0) {
      ppuVar4 = &PTR_017f8e10;
    }
    else {
      ppuVar4 = (undefined **)*puVar3;
    }
    (**(code **)(*(int *)this + 0x11c))(ppuVar4);
    puStack_8 = (undefined1 *)0xffffffff;
    FUN_0041b420((int *)&stack0xffffff3c);
    (**(code **)(*(int *)this + 0x120))();
  }
  else {
    piVar2[0x1a] = (int)this;
    *(undefined4 *)(*(int *)((int)this + 0x468) + 0x270) = *(undefined4 *)((int)this + 0x40);
    FUN_00957550(local_b4,*(int *)((int)this + 0x468),0);
    local_4 = 1;
    iVar1 = *(int *)((int)this + 0x44);
    FUN_0047f0e0(local_b4,iVar1 + 0xc,4);
    FUN_0047f0e0(local_b4,iVar1 + 0x10,4);
    FUN_0047f0e0(local_b4,iVar1 + 0x14,4);
    FUN_0047f0e0(local_b4,iVar1 + 0x18,4);
    local_e = 1;
    if (local_88 != 0) {
      FUN_00486000("!Bunch.IsError()",".\\Src\\UnDownload.cpp",0xf9);
    }
    FUN_008c8e80(*(void **)((int)this + 0x468),local_b4,0);
    local_4 = 0xffffffff;
    FUN_005e1190(local_b4);
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] UChannelDownload virtual function #1
   VTable: vtable_UChannelDownload at 018e7dac */

bool __fastcall UChannelDownload__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee64c8 == (undefined4 *)0x0) {
    DAT_01ee64c8 = FUN_008c4ee0();
    FUN_008c3fc0();
  }
  return puVar1 != DAT_01ee64c8;
}


/* [VTable] UChannelDownload virtual function #2
   VTable: vtable_UChannelDownload at 018e7dac */

int * __thiscall UChannelDownload__vfunc_2(void *this,byte param_1)

{
  FUN_008c6190(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UChildConnection.c ========== */

/*
 * SGW.exe - UChildConnection (13 functions)
 */

/* [VTable] UChildConnection virtual function #78
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_78(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x005ddd7e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(**(int **)(param_1 + 0x4f58) + 0x138))();
  return;
}


/* [VTable] UChildConnection virtual function #79
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_79(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x005ddd8e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(**(int **)(param_1 + 0x4f58) + 0x13c))();
  return;
}


/* [VTable] UChildConnection virtual function #81
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_81(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x005ddd9e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(**(int **)(param_1 + 0x4f58) + 0x144))();
  return;
}


/* [VTable] UChildConnection virtual function #70
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_70(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x005dddae. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(**(int **)(param_1 + 0x4f58) + 0x118))();
  return;
}


/* [VTable] UChildConnection virtual function #82
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_82(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x005dddbe. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(**(int **)(param_1 + 0x4f58) + 0x148))();
  return;
}


/* [VTable] UChildConnection virtual function #69
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_69(int param_1)

{
  *(undefined4 *)(param_1 + 0x68) = *(undefined4 *)(*(int *)(param_1 + 0x4f58) + 0x68);
  return;
}


/* [VTable] UChildConnection virtual function #67
   VTable: vtable_UChildConnection at 01848024 */

undefined4 __thiscall UChildConnection__vfunc_67(void *this,undefined4 param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01677f39;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  (**(code **)(**(int **)((int)this + 0x4f58) + 0x10c))(param_1);
  ExceptionList = (void *)0x0;
  return param_1;
}


/* [VTable] UChildConnection virtual function #68
   VTable: vtable_UChildConnection at 01848024 */

undefined4 __thiscall UChildConnection__vfunc_68(void *this,undefined4 param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01677f39;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  (**(code **)(**(int **)((int)this + 0x4f58) + 0x110))(param_1);
  ExceptionList = (void *)0x0;
  return param_1;
}


/* [VTable] UChildConnection virtual function #2
   VTable: vtable_UChildConnection at 01848024 */

int * __thiscall UChildConnection__vfunc_2(void *this,byte param_1)

{
  FUN_005ddea0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UChildConnection virtual function #72
   VTable: vtable_UChildConnection at 01848024 */

void __fastcall UChildConnection__vfunc_72(int param_1)

{
  if ((DAT_01ead7d0 != 0) && (*(int *)(param_1 + 0x40) != 0)) {
    *(undefined4 *)(*(int *)(param_1 + 0x40) + 0x2ac) = 0;
    if (DAT_01ee2684 != (void *)0x0) {
      FUN_00875290(DAT_01ee2684,*(int **)(param_1 + 0x40),1,1);
    }
    *(undefined4 *)(param_1 + 0x40) = 0;
  }
  *(undefined4 *)(param_1 + 0x60) = 0;
  *(undefined4 *)(param_1 + 100) = 0;
  return;
}


/* Also in vtable: UChildConnection (slot 0) */

undefined4 UChildConnection__vfunc_0(void)

{
  return 1;
}


/* [VTable] UChildConnection virtual function #73
   VTable: vtable_UChildConnection at 01848024 */

void __thiscall UChildConnection__vfunc_73(void *this,void *param_1)

{
  int iVar1;
  int iVar2;
  int iVar3;
  bool bVar4;
  int iVar5;
  int iVar6;
  int *piVar7;
  int iVar8;
  int local_8;
  int local_4;
  
  iVar5 = FUN_00419580(DAT_01ee1254);
  FUN_0053c9a0(&local_8,iVar5);
  do {
    iVar5 = local_4;
    if (local_8 == 0) {
      return;
    }
    if (local_4 < 0) {
      return;
    }
    iVar1 = *(int *)(local_8 + 0x2d4);
    if (iVar1 <= local_4) {
      return;
    }
    iVar2 = *(int *)(local_8 + 0x2d0);
    iVar8 = *(int *)(iVar2 + local_4 * 4);
    if (*(int *)(iVar8 + 0x40) == 0) {
      if (iVar1 <= local_4) {
        return;
      }
      iVar5 = *(int *)(iVar2 + local_4 * 4);
      goto LAB_005deea1;
    }
    iVar3 = *(int *)((int)this + 0x4f58);
    if (*(int *)(iVar3 + 0x40) != *(int *)(iVar8 + 0x40)) {
      bVar4 = false;
      if (0 < *(int *)(iVar3 + 0x4f24)) {
        iVar6 = FUN_00539320(&local_8);
        piVar7 = *(int **)(iVar3 + 0x4f20);
        iVar8 = *(int *)(*(int *)((int)this + 0x4f58) + 0x4f24);
        do {
          if (*(int *)(*piVar7 + 0x40) == *(int *)(iVar6 + 0x40)) {
            bVar4 = true;
          }
          piVar7 = piVar7 + 1;
          iVar8 = iVar8 + -1;
        } while (iVar8 != 0);
        if (bVar4) goto LAB_005dee60;
      }
      if (*(int *)(local_8 + 0x2d4) <= iVar5) {
        return;
      }
      iVar5 = *(int *)(*(int *)(local_8 + 0x2d0) + iVar5 * 4);
LAB_005deea1:
      if (iVar5 == 0) {
        return;
      }
      if (*(int *)(iVar5 + 0x40) != 0) {
        *(undefined4 *)(*(int *)(iVar5 + 0x40) + 0x2ac) = 0;
        FUN_00875290(DAT_01ee2684,*(int **)(iVar5 + 0x40),0,1);
        *(undefined4 *)(iVar5 + 0x40) = 0;
      }
      *(undefined4 *)(iVar5 + 0x44) = *(undefined4 *)((int)this + 0x44);
      *(undefined1 *)((int)param_1 + 0x5a) = 2;
      FUN_0076c0f0(param_1,iVar5);
      *(void **)((int)this + 0x40) = param_1;
      return;
    }
LAB_005dee60:
    do {
      iVar5 = iVar5 + 1;
      local_4 = iVar5;
      if ((iVar5 < 0) || (iVar1 <= iVar5)) break;
    } while (*(int *)(iVar2 + iVar5 * 4) == 0);
  } while( true );
}


/* [VTable] UChildConnection virtual function #1
   VTable: vtable_UChildConnection at 01848024 */

bool __fastcall UChildConnection__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2bc0 == (undefined4 *)0x0) {
    DAT_01ee2bc0 = FUN_005e0da0();
    FUN_005de540();
  }
  return puVar1 != DAT_01ee2bc0;
}




/* ========== UControlChannel.c ========== */

/*
 * SGW.exe - UControlChannel (7 functions)
 */

/* [VTable] UControlChannel virtual function #67
   VTable: vtable_UControlChannel at 018e8de4 */

void __thiscall UControlChannel__vfunc_67(void *this,int *param_1,undefined4 param_2,int param_3)

{
  int iVar1;
  
  iVar1 = (**(code **)(*param_1 + 0x130))();
  if (iVar1 == 0) {
    *(int **)((int)this + 0x3c) = param_1;
  }
  else {
    *(int *)((int)this + 0x3c) = param_1[0x13d6];
  }
  *(undefined4 *)((int)this + 0x44) = param_2;
  *(int *)((int)this + 0x48) = param_3;
  *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
  *(int *)((int)this + 0x5c) = param_1[0x34];
  if (param_3 == 0) {
    *(undefined4 *)((int)this + 0x6c) = 1;
  }
  return;
}


/* Also in vtable: UControlChannel (slot 0) */

undefined4 UControlChannel__vfunc_0(void)

{
  return 1;
}


/* [VTable] UControlChannel virtual function #1
   VTable: vtable_UControlChannel at 018e8de4 */

bool __fastcall UControlChannel__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee64f0 == (undefined4 *)0x0) {
    DAT_01ee64f0 = FUN_008c7980();
    FUN_008c7a50();
  }
  return puVar1 != DAT_01ee64f0;
}


/* [VTable] UControlChannel virtual function #71
   VTable: vtable_UControlChannel at 018e8de4 */

void __thiscall UControlChannel__vfunc_71(void *this,int *param_1)

{
  int iVar1;
  void *extraout_ECX;
  void *extraout_ECX_00;
  void *this_00;
  void *extraout_ECX_01;
  undefined **ppuVar2;
  undefined **local_18;
  int local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdb0d;
  local_c = ExceptionList;
  this_00 = this;
  ExceptionList = &local_c;
  if ((*(byte *)((int)this + 0x40) & 2) != 0) {
    ExceptionList = &local_c;
    FUN_00486000("!Closing",".\\Src\\UnChan.cpp",0x237);
    this_00 = extraout_ECX;
  }
  if ((*(int *)((int)this + 0x6c) == 1) &&
     (iVar1 = FUN_008c7bb0(this,(int)param_1), this_00 = extraout_ECX_00, iVar1 == 0)) {
    (**(code **)(**(int **)((int)this + 0x3c) + 0x11c))();
    ExceptionList = local_c;
    return;
  }
  while( true ) {
    local_18 = (undefined **)0x0;
    local_14 = 0;
    local_10 = 0;
    local_4 = 2;
    FUN_00485df0(this_00,param_1,(int *)&local_18);
    if (param_1[0xb] != 0) {
      local_4 = 0xffffffff;
      FUN_0041b420((int *)&local_18);
      ExceptionList = local_c;
      return;
    }
    ppuVar2 = local_18;
    if (local_14 == 0) {
      ppuVar2 = &PTR_017f8e10;
    }
    (**(code **)(**(int **)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0x54) + 0x10))
              (*(int *)((int)this + 0x3c),ppuVar2);
    local_4 = 0xffffffff;
    FUN_0041b420((int *)&local_18);
    if (*(int *)((int)this + 0x3c) == 0) break;
    this_00 = extraout_ECX_01;
    if (*(int *)(*(int *)((int)this + 0x3c) + 0x68) == 1) {
      ExceptionList = local_c;
      return;
    }
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] UControlChannel virtual function #70
   VTable: vtable_UControlChannel at 018e8de4 */

void * __thiscall UControlChannel__vfunc_70(void *this,void *param_1)

{
  undefined4 *puVar1;
  undefined4 *this_00;
  int local_24 [3];
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdb5e;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = UChannel__vfunc_70(this,local_18);
  local_4 = 1;
  this_00 = FUN_0041aab0(local_24,L"Text ");
  local_4._0_1_ = 2;
  FUN_0041b600(this_00,param_1,puVar1);
  local_4._0_1_ = 1;
  FUN_0041b420(local_24);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_0041b420(local_18);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] UControlChannel virtual function #73
   VTable: vtable_UControlChannel at 018e8de4 */

void __fastcall UControlChannel__vfunc_73(void *param_1)

{
  int iVar1;
  undefined **local_b4 [11];
  int local_88;
  int local_48 [14];
  undefined1 local_e;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdcb5;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  UChannel__vfunc_73((int)param_1);
  iVar1 = *(int *)((int)param_1 + 0x74);
  while( true ) {
    if (iVar1 < 1) {
      ExceptionList = local_c;
      return;
    }
    if ((*(byte *)((int)param_1 + 0x40) & 2) != 0) {
      ExceptionList = local_c;
      return;
    }
    FUN_00957550(local_b4,(int)param_1,0);
    local_4 = 0;
    if (local_88 != 0) break;
    local_e = 1;
    FUN_00485df0(local_b4,(int *)local_b4,*(int **)((int)param_1 + 0x70));
    if (local_88 != 0) {
      (**(code **)(**(int **)((int)param_1 + 0x3c) + 0x11c))();
      break;
    }
    FUN_008c8e80(param_1,local_b4,1);
    FUN_0041baf0((void *)((int)param_1 + 0x70),0,1);
    local_4 = 2;
    FUN_0048ead0(local_48);
    local_4 = 0xffffffff;
    iVar1 = *(int *)((int)param_1 + 0x74);
    local_b4[0] = FArchive::vftable;
  }
  local_4 = 0xffffffff;
  FUN_005e1190(local_b4);
  ExceptionList = local_c;
  return;
}


/* [VTable] UControlChannel virtual function #2
   VTable: vtable_UControlChannel at 018e8de4 */

int * __thiscall UControlChannel__vfunc_2(void *this,byte param_1)

{
  FUN_008cc100(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UDownload.c ========== */

/*
 * SGW.exe - UDownload (7 functions)
 */

/* [VTable] UDownload virtual function #12
   VTable: vtable_UDownload at 018e7c84 */

void __thiscall UDownload__vfunc_12(void *this,int *param_1)

{
  UTestIpDrv__vfunc_12(this,param_1);
  (**(code **)(*param_1 + 0x18))((int)this + 0x3c);
  return;
}


/* [VTable] UDownload virtual function #67
   VTable: vtable_UDownload at 018e7c84 */

undefined4 __fastcall UDownload__vfunc_67(int param_1)

{
  if ((*(int *)(param_1 + 0x58) != 0) && ((*(byte *)(*(int *)(param_1 + 0x44) + 0x30) & 2) != 0)) {
    *(undefined4 *)(param_1 + 0x460) = 1;
    return 1;
  }
  return 0;
}


/* [VTable] UDownload virtual function #11
   VTable: vtable_UDownload at 018e7c84 */

void __fastcall UDownload__vfunc_11(int param_1)

{
  int iVar1;
  
  if (*(undefined4 **)(param_1 + 0x58) != (undefined4 *)0x0) {
    (**(code **)**(undefined4 **)(param_1 + 0x58))(1);
    *(undefined4 *)(param_1 + 0x58) = 0;
    (**(code **)(*(int *)PTR_PTR_01dae7f8 + 0x1c))(param_1 + 0x5c,0,0);
  }
  iVar1 = *(int *)(param_1 + 0x3c);
  if ((iVar1 != 0) && (*(int *)(iVar1 + 0x4ee4) == param_1)) {
    *(undefined4 *)(iVar1 + 0x4ee4) = 0;
  }
  *(undefined4 *)(param_1 + 0x3c) = 0;
  UTestIpDrv__vfunc_11(param_1);
  return;
}


/* [VTable] UDownload virtual function #68
   VTable: vtable_UDownload at 018e7c84 */

void __thiscall UDownload__vfunc_68(void *this,int param_1,int param_2)

{
  *(int *)((int)this + 0x3c) = param_1;
  *(int *)((int)this + 0x40) = param_2;
  *(int *)((int)this + 0x44) = *(int *)(*(int *)(param_1 + 0x60) + 0x3c) + param_2 * 0x3c;
  return;
}


/* Also in vtable: UDownload (slot 0) */

undefined4 UDownload__vfunc_0(void)

{
  return 1;
}


/* [VTable] UDownload virtual function #1
   VTable: vtable_UDownload at 018e7c84 */

bool __fastcall UDownload__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UDownload virtual function #2
   VTable: vtable_UDownload at 018e7c84 */

int * __thiscall UDownload__vfunc_2(void *this,byte param_1)

{
  FUN_008c6060(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UFileChannel.c ========== */

/*
 * SGW.exe - UFileChannel (8 functions)
 */

/* [VTable] UFileChannel virtual function #67
   VTable: vtable_UFileChannel at 018e89cc */

void __thiscall
UFileChannel__vfunc_67(void *this,int *param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  
  iVar1 = (**(code **)(*param_1 + 0x130))();
  if (iVar1 == 0) {
    *(int **)((int)this + 0x3c) = param_1;
  }
  else {
    *(int *)((int)this + 0x3c) = param_1[0x13d6];
  }
  *(undefined4 *)((int)this + 0x44) = param_2;
  *(undefined4 *)((int)this + 0x48) = param_3;
  *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
  *(int *)((int)this + 0x5c) = param_1[0x34];
  return;
}


/* Also in vtable: UFileChannel (slot 0) */

undefined4 UFileChannel__vfunc_0(void)

{
  return 1;
}


/* [VTable] UFileChannel virtual function #74
   VTable: vtable_UFileChannel at 018e89cc */

void __fastcall UFileChannel__vfunc_74(int param_1)

{
  if (*(undefined4 **)(param_1 + 0x6c) != (undefined4 *)0x0) {
    (**(code **)**(undefined4 **)(param_1 + 0x6c))(1);
    *(undefined4 *)(param_1 + 0x6c) = 0;
  }
  if ((*(int *)(param_1 + 0x48) != 0) && (*(int **)(param_1 + 0x68) != (int *)0x0)) {
    (**(code **)(**(int **)(param_1 + 0x68) + 0x120))();
    FUN_008c3dd0(*(int *)(param_1 + 0x68));
  }
  UChannel__vfunc_74(param_1);
  return;
}


/* [VTable] UFileChannel virtual function #1
   VTable: vtable_UFileChannel at 018e89cc */

bool __fastcall UFileChannel__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee64f0 == (undefined4 *)0x0) {
    DAT_01ee64f0 = FUN_008c7980();
    FUN_008c7a50();
  }
  return puVar1 != DAT_01ee64f0;
}


/* [VTable] UFileChannel virtual function #70
   VTable: vtable_UFileChannel at 018e89cc */

void * __thiscall UFileChannel__vfunc_70(void *this,void *param_1)

{
  undefined4 *puVar1;
  void *this_00;
  int local_24 [3];
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdbf7;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = UChannel__vfunc_70(this,local_18);
  local_4 = 1;
  this_00 = FUN_00bb4780(local_24);
  local_4._0_1_ = 2;
  FUN_0041b600(this_00,param_1,puVar1);
  local_4._0_1_ = 1;
  FUN_0041b420(local_24);
  local_4 = (uint)local_4._1_3_ << 8;
  FUN_0041b420(local_18);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] UFileChannel virtual function #71
   VTable: vtable_UFileChannel at 018e89cc */

void __thiscall UFileChannel__vfunc_71(void *this,int *param_1)

{
  bool bVar1;
  bool bVar2;
  undefined4 uVar3;
  undefined3 extraout_var;
  int iVar4;
  undefined4 *puVar5;
  undefined **ppuVar6;
  void *this_00;
  int iVar7;
  int *this_01;
  int local_188;
  int local_184 [3];
  int local_178;
  int local_174;
  int local_170;
  int local_16c;
  int aiStack_168 [3];
  undefined4 local_15c [42];
  undefined4 local_b4 [42];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bde0b;
  local_c = ExceptionList;
  iVar7 = 0;
  bVar2 = false;
  ExceptionList = &local_c;
  if ((*(byte *)((int)this + 0x40) & 2) != 0) {
    ExceptionList = &local_c;
    FUN_00486000("!Closing",".\\Src\\UnChan.cpp",0x5d6);
  }
  if (*(int *)((int)this + 0x48) == 0) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0x8c) == 0) {
      FUN_00957550(local_15c,(int)this,1);
      local_4 = 0;
      FUN_008c8e80(this,local_15c,0);
      puVar5 = local_15c;
    }
    else {
      if (*(int *)((int)this + 0x6c) == 0) {
        FUN_004b6970(param_1,(int)&local_178);
        if ((param_1[0xb] == 0) &&
           (local_188 = 0, 0 < *(int *)(*(int *)(*(int *)((int)this + 0x3c) + 0x60) + 0x40))) {
          do {
            iVar4 = *(int *)(*(int *)(*(int *)((int)this + 0x3c) + 0x60) + 0x3c);
            this_01 = (int *)(iVar4 + iVar7);
            if ((((*(int *)(iVar4 + 0x10 + iVar7) == local_174 &&
                  *(int *)(iVar4 + 0x14 + iVar7) == local_170) && this_01[6] == local_16c) &&
                 this_01[3] == local_178) && ((*this_01 != 0 || (this_01[1] != 0)))) {
              if (*(int *)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0xa4) < 1) {
LAB_008cb219:
                bVar1 = false;
              }
              else {
                FUN_0049b190(this_01,local_184);
                local_4 = 2;
                bVar2 = true;
                iVar4 = (**(code **)(*(int *)PTR_PTR_01dae7f8 + 0xc))();
                if (iVar4 <= *(int *)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0xa4))
                goto LAB_008cb219;
                bVar1 = true;
              }
              local_4 = 0xffffffff;
              if (bVar2) {
                bVar2 = false;
                FUN_0041b420(local_184);
              }
              if (bVar1) break;
              puVar5 = FUN_0049b190(this_01,aiStack_168);
              local_4 = 3;
              if (puVar5[1] == 0) {
                ppuVar6 = &PTR_017f8e10;
              }
              else {
                ppuVar6 = (undefined **)*puVar5;
              }
              FUN_00490d80((wchar_t *)((int)this + 0x70),(wchar_t *)ppuVar6,0x100);
              local_4 = 0xffffffff;
              FUN_0041b420(aiStack_168);
              iVar4 = (**(code **)(**(int **)(*(int *)(*(int *)((int)this + 0x3c) + 100) + 0x54) +
                                  0x14))(*(int *)((int)this + 0x3c),CONCAT44(local_174,local_178),
                                         CONCAT44(local_16c,local_170));
              if (iVar4 != 0) {
                iVar4 = (**(code **)(*(int *)PTR_PTR_01dae7f8 + 4))();
                *(int *)((int)this + 0x6c) = iVar4;
                if (iVar4 != 0) {
                  *(int *)((int)this + 0x270) = local_188;
                  ExceptionList = local_c;
                  return;
                }
              }
            }
            local_188 = local_188 + 1;
            iVar7 = iVar7 + 0x3c;
          } while (local_188 < *(int *)(*(int *)(*(int *)((int)this + 0x3c) + 0x60) + 0x40));
        }
      }
      else {
        FUN_007fa050(local_184);
        local_4 = 1;
        FUN_00485df0(this_00,param_1,local_184);
        if ((param_1[0xb] == 0) &&
           (bVar2 = USeqVar_Union__unknown_004195f0(local_184,L"SKIP"),
           CONCAT31(extraout_var,bVar2) != 0)) {
          FUN_0047f620((void *)(*(int *)(*(int *)((int)this + 0x3c) + 0x60) + 0x3c),
                       *(int *)((int)this + 0x270),1);
          local_4 = 0xffffffff;
          FUN_0041b420(local_184);
          ExceptionList = local_c;
          return;
        }
        local_4 = 0xffffffff;
        FUN_0041b420(local_184);
      }
      FUN_00957550(local_b4,(int)this,1);
      local_4 = 4;
      FUN_008c8e80(this,local_b4,0);
      puVar5 = local_b4;
    }
    local_4 = 0xffffffff;
    FUN_005e1190(puVar5);
  }
  else {
    iVar7 = **(int **)((int)this + 0x68);
    FUN_0156bb10((int)param_1);
    uVar3 = FUN_0156c140((int)param_1);
    (**(code **)(iVar7 + 0x114))(uVar3);
  }
  ExceptionList = local_c;
  return;
}


/* WARNING: Function: __alloca_probe_16 replaced with injection: alloca_probe */
/* [VTable] UFileChannel virtual function #73
   VTable: vtable_UFileChannel at 018e89cc */

void __fastcall UFileChannel__vfunc_73(void *param_1)

{
  uint uVar1;
  wchar_t *pwVar2;
  int iVar3;
  int iVar4;
  undefined1 *puVar5;
  wchar_t *pwVar6;
  undefined **local_bc [11];
  int local_90;
  int local_50 [14];
  char local_17;
  undefined1 local_16;
  int local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_016bde4c;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  UChannel__vfunc_73((int)param_1);
  uVar1 = DAT_01ee6504 & 1;
  *(undefined4 *)(*(int *)((int)param_1 + 0x3c) + 0x130) = 1;
  if (uVar1 == 0) {
    DAT_01ee6504 = DAT_01ee6504 | 1;
    local_8 = 0;
    pwVar6 = L"lanplay";
    pwVar2 = (wchar_t *)FUN_00483290();
    DAT_01ee6500 = FUN_00484390(pwVar2,pwVar6);
    local_8 = 0xffffffff;
  }
  iVar3 = *(int *)((int)param_1 + 0x48);
  while ((((iVar3 == 0 && (*(int *)((int)param_1 + 0x6c) != 0)) &&
          (*(int *)((int)param_1 + 0x58) < 0x7f)) &&
         (iVar3 = (**(code **)(**(int **)((int)param_1 + 0x3c) + 0x118))(), iVar3 != 0))) {
    iVar3 = *(int *)((int)param_1 + 0x3c);
    iVar4 = ((*(int *)(iVar3 + 0xb0) * 8 -
             ((-(uint)(*(int *)(iVar3 + 0x29c) != 0) & 0xfffffff0) + 0x10)) -
            *(int *)(iVar3 + 0x29c)) + -0x41;
    iVar4 = (int)(iVar4 + (iVar4 >> 0x1f & 7U)) >> 3;
    if (iVar4 < 1) {
      ExceptionList = local_10;
      return;
    }
    iVar3 = *(int *)(*(int *)(*(int *)(iVar3 + 0x60) + 0x3c) + 0x1c +
                    *(int *)((int)param_1 + 0x270) * 0x3c) - *(int *)((int)param_1 + 0x274);
    local_14 = iVar4;
    FUN_00957550(local_bc,(int)param_1,iVar3 <= iVar4);
    local_8 = 1;
    if (iVar4 <= iVar3) {
      iVar3 = iVar4;
    }
    if (iVar3 == 0) {
      puVar5 = (undefined1 *)0x0;
    }
    else {
      puVar5 = &stack0xffffff34;
    }
    (**(code **)(**(int **)((int)param_1 + 0x6c) + 4))(puVar5,iVar3);
    *(int *)((int)param_1 + 0x274) = *(int *)((int)param_1 + 0x274) + iVar3;
    FBitWriter__vfunc_1(local_bc,(int)puVar5,iVar3);
    local_16 = 1;
    if (local_90 != 0) {
      FUN_00486000("!Bunch.IsError()",".\\Src\\UnChan.cpp",0x632);
    }
    FUN_008c8e80(param_1,local_bc,0);
    (**(code **)(**(int **)((int)param_1 + 0x3c) + 0x144))();
    if (local_17 != '\0') {
      if (*(undefined4 **)((int)param_1 + 0x6c) != (undefined4 *)0x0) {
        (**(code **)**(undefined4 **)((int)param_1 + 0x6c))();
      }
      *(undefined4 *)((int)param_1 + 0x6c) = 0;
    }
    local_8 = 3;
    FUN_0048ead0(local_50);
    local_8 = 0xffffffff;
    iVar3 = *(int *)((int)param_1 + 0x48);
    local_bc[0] = FArchive::vftable;
  }
  ExceptionList = local_10;
  return;
}


/* [VTable] UFileChannel virtual function #2
   VTable: vtable_UFileChannel at 018e89cc */

int * __thiscall UFileChannel__vfunc_2(void *this,byte param_1)

{
  FUN_008cb7c0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UGameEntityManagerGCHook.c ========== */

/*
 * SGW.exe - UGameEntityManagerGCHook (1 functions)
 */

undefined4 UGameEntityManagerGCHook__vfunc_0(void)

{
  return 1;
}




/* ========== UNetConnection.c ========== */

/*
 * SGW.exe - UNetConnection (3 functions)
 */

/* [VTable] UNetConnection virtual function #2
   VTable: vtable_UNetConnection at 01800ec4 */

int * __thiscall UNetConnection__vfunc_2(void *this,byte param_1)

{
  FUN_0047fd60(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* Also in vtable: UNetConnection (slot 0) */

undefined4 UNetConnection__vfunc_0(void)

{
  return 1;
}


/* [VTable] UNetConnection virtual function #1
   VTable: vtable_UNetConnection at 01800ec4 */

bool __fastcall UNetConnection__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2b40 == (undefined4 *)0x0) {
    DAT_01ee2b40 = FUN_005d32b0();
    FUN_005d21e0();
  }
  return puVar1 != DAT_01ee2b40;
}




/* ========== UNetConnectionBase.c ========== */

/*
 * SGW.exe - UNetConnectionBase (3 functions)
 */

/* [VTable] UNetConnectionBase virtual function #2
   VTable: vtable_UNetConnectionBase at 01800d74 */

int * __thiscall UNetConnectionBase__vfunc_2(void *this,byte param_1)

{
  FUN_0047fbe0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* Also in vtable: UNetConnectionBase (slot 0) */

undefined4 UNetConnectionBase__vfunc_0(void)

{
  return 1;
}


/* [VTable] UNetConnectionBase virtual function #1
   VTable: vtable_UNetConnectionBase at 01800d74 */

bool __fastcall UNetConnectionBase__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2b40 == (undefined4 *)0x0) {
    DAT_01ee2b40 = FUN_005d32b0();
    FUN_005d21e0();
  }
  return puVar1 != DAT_01ee2b40;
}




/* ========== UNetDriver.c ========== */

/*
 * SGW.exe - UNetDriver (4 functions)
 */

/* [VTable] UNetDriver virtual function #2
   VTable: vtable_UNetDriver at 018011a4 */

int * __thiscall UNetDriver__vfunc_2(void *this,byte param_1)

{
  FUN_004800c0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UNetDriver virtual function #73
   VTable: vtable_UNetDriver at 018011a4 */

undefined4 __thiscall UNetDriver__vfunc_73(void *this,int param_1)

{
  (**(code **)(*(int *)this + 0x110))();
  *(int *)((int)this + 0x54) = param_1;
  return 1;
}


/* Also in vtable: UNetDriver (slot 0) */

undefined4 UNetDriver__vfunc_0(void)

{
  return 1;
}


/* [VTable] UNetDriver virtual function #1
   VTable: vtable_UNetDriver at 018011a4 */

bool __fastcall UNetDriver__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ec69a8 == (undefined4 *)0x0) {
    DAT_01ec69a8 = FUN_004957d0();
    FUN_00494b80();
  }
  return puVar1 != DAT_01ec69a8;
}




/* ========== UNetPendingLevel.c ========== */

/*
 * SGW.exe - UNetPendingLevel (7 functions)
 */

/* [VTable] UNetPendingLevel virtual function #69
   VTable: vtable_UNetPendingLevel at 018e969c */

void UNetPendingLevel__vfunc_69(void)

{
  return;
}


/* [VTable] UNetPendingLevel virtual function #70
   VTable: vtable_UNetPendingLevel at 018e969c */

undefined4 __fastcall UNetPendingLevel__vfunc_70(int param_1)

{
  int iVar1;
  
  iVar1 = FUN_00554590(*(int *)(*(int *)(param_1 + 0x90) + 0x50));
  if ((iVar1 != 0) && (*(int *)(iVar1 + 0x4ee4) != 0)) {
    iVar1 = (**(code **)(**(int **)(iVar1 + 0x4ee4) + 0x10c))();
    if (iVar1 != 0) {
      return 1;
    }
  }
  return 0;
}


/* Also in vtable: UNetPendingLevel (slot 0) */

undefined4 UNetPendingLevel__vfunc_0(void)

{
  return 1;
}


/* [VTable] UNetPendingLevel virtual function #67
   VTable: vtable_UNetPendingLevel at 018e969c */

void __thiscall UNetPendingLevel__vfunc_67(void *this,undefined4 param_1)

{
  undefined **_Str1;
  int iVar1;
  int *piVar2;
  int aiStack_18 [2];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016bdff8;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (*(int *)((int)this + 0x90) == 0) {
    ExceptionList = &pvStack_c;
    FUN_00486000("NetDriver",".\\Src\\UnPenLev.cpp",0x15c);
  }
  if (*(int *)(*(int *)((int)this + 0x90) + 0x50) == 0) {
    FUN_00486000("NetDriver->ServerConnection",".\\Src\\UnPenLev.cpp",0x15d);
  }
  if (*(int *)(*(int *)(*(int *)((int)this + 0x90) + 0x50) + 0x68) == 1) {
    if (*(int *)((int)this + 0xa4) == 0) {
      _Str1 = &PTR_017f8e10;
    }
    else {
      _Str1 = *(undefined ***)((int)this + 0xa0);
    }
    iVar1 = _wcsicmp((wchar_t *)_Str1,L"");
    if (iVar1 == 0) {
      piVar2 = FUN_00489340(aiStack_18,L"ConnectionFailed",L"Engine",(wchar_t *)0x0);
      uStack_4 = 0;
      FUN_0041a630((undefined4 *)((int)this + 0xa0),piVar2);
      uStack_4 = 0xffffffff;
      FUN_0041b420(aiStack_18);
      ExceptionList = pvStack_c;
      return;
    }
  }
  (**(code **)(**(int **)((int)this + 0x90) + 300))(param_1);
  (**(code **)(**(int **)((int)this + 0x90) + 0x128))();
  ExceptionList = pvStack_10;
  return;
}


/* [VTable] UNetPendingLevel virtual function #1
   VTable: vtable_UNetPendingLevel at 018e969c */

bool __fastcall UNetPendingLevel__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee650c == (undefined4 *)0x0) {
    DAT_01ee650c = FUN_008ccd10();
    FUN_008cc8f0();
  }
  return puVar1 != DAT_01ee650c;
}


/* [VTable] UNetPendingLevel virtual function #68
   VTable: vtable_UNetPendingLevel at 018e969c */

undefined4 __fastcall UNetPendingLevel__vfunc_68(int param_1)

{
  return *(undefined4 *)(param_1 + 0x90);
}


/* [VTable] UNetPendingLevel virtual function #2
   VTable: vtable_UNetPendingLevel at 018e969c */

int * __thiscall UNetPendingLevel__vfunc_2(void *this,byte param_1)

{
  FUN_008cd350(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UTcpNetDriver.c ========== */

/*
 * SGW.exe - UTcpNetDriver (8 functions)
 */

/* [VTable] UTcpNetDriver virtual function #69
   VTable: vtable_UTcpNetDriver at 018012fc */

void __fastcall UTcpNetDriver__vfunc_69(int param_1)

{
  if ((*(int **)(param_1 + 0x148) != (int *)0x0) && ((*(uint *)(param_1 + 8) & 0x200) == 0)) {
    (**(code **)(**(int **)(param_1 + 0x148) + 4))();
    *(undefined4 *)(param_1 + 0x148) = 0;
  }
  return;
}


/* Also in vtable: UTcpNetDriver (slot 0) */

undefined4 UTcpNetDriver__vfunc_0(void)

{
  return 1;
}


/* [VTable] UTcpNetDriver virtual function #1
   VTable: vtable_UTcpNetDriver at 018012fc */

bool __fastcall UTcpNetDriver__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2bb8 == (undefined4 *)0x0) {
    DAT_01ee2bb8 = FUN_005dd860();
    FUN_005dc080();
  }
  return puVar1 != DAT_01ee2bb8;
}


/* [VTable] UTcpNetDriver virtual function #70
   VTable: vtable_UTcpNetDriver at 018012fc */

void * __thiscall UTcpNetDriver__vfunc_70(void *this,void *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167ec87;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  UTcpipConnection__unknown_0047f7a0((void *)((int)this + 0x138),param_1,0);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] UTcpNetDriver virtual function #72
   VTable: vtable_UTcpNetDriver at 018012fc */

undefined4 __thiscall UTcpNetDriver__vfunc_72(void *this,int param_1,void *param_2)

{
  u_short uVar1;
  int iVar2;
  int *piVar3;
  undefined8 local_10;
  undefined8 local_8;
  
  iVar2 = UNetDriver__vfunc_72(this,param_1);
  if (iVar2 != 0) {
    iVar2 = FUN_0047e600(this,1,param_1,param_2);
    if (iVar2 != 0) {
      local_8 = 0;
      local_10 = 2;
      uVar1 = htons((u_short)*(undefined4 *)((int)param_2 + 0x18));
      local_10 = (ulonglong)CONCAT22(uVar1,(undefined2)local_10);
      piVar3 = (int *)T_StaticClass_3(*(undefined4 **)((int)this + 0x108),-1,0,0,0,0,0,0,0);
      *(int **)((int)this + 0x50) = piVar3;
      (**(code **)(*piVar3 + 300))
                (this,*(undefined4 *)((int)this + 0x148),&local_10,2,1,param_2,0,0);
      Mercury_Channel_7(*(void **)((int)this + 0x50),1,1,0);
      return 1;
    }
  }
  return 0;
}


/* [VTable] UTcpNetDriver virtual function #73
   VTable: vtable_UTcpNetDriver at 018012fc */

undefined4 __thiscall UTcpNetDriver__vfunc_73(void *this,int param_1,void *param_2)

{
  u_short uVar1;
  int iVar2;
  int *piVar3;
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167ecc3;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar2 = UNetDriver__vfunc_73(this,param_1);
  if (iVar2 != 0) {
    iVar2 = FUN_0047e600(this,0,param_1,param_2);
    if (iVar2 != 0) {
      piVar3 = UTcpipConnection__unknown_0047f7a0((void *)((int)this + 0x138),local_18,0);
      local_4 = 0;
      FUN_0041a630((void *)((int)param_2 + 0xc),piVar3);
      local_4 = 0xffffffff;
      FUN_0041b420(local_18);
      uVar1 = ntohs(*(u_short *)((int)this + 0x13a));
      *(uint *)((int)param_2 + 0x18) = (uint)uVar1;
      ExceptionList = local_c;
      return 1;
    }
  }
  ExceptionList = local_c;
  return 0;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] UTcpNetDriver virtual function #75
   VTable: vtable_UTcpNetDriver at 018012fc */

void __thiscall UTcpNetDriver__vfunc_75(void *this,float param_1)

{
  undefined4 *puVar1;
  int *piVar2;
  int *piVar3;
  int iVar4;
  int iVar5;
  undefined4 uVar6;
  int *piVar7;
  undefined1 *puStack_2b0;
  undefined4 local_2ac;
  undefined8 local_2a8;
  undefined8 local_2a0;
  LARGE_INTEGER local_298;
  undefined1 auStack_290 [620];
  undefined4 uStack_24;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167ed10;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  UNetDriver__vfunc_75(this,param_1);
  local_2a0 = 0;
  local_2a8 = 2;
LAB_0047ed58:
  do {
    local_2ac = 0;
    QueryPerformanceCounter(&local_298);
    *(int *)((int)this + 0xa0) = *(int *)((int)this + 0xa0) - local_298._0_4_;
    puStack_2b0 = (undefined1 *)(**(code **)(**(int **)((int)this + 0x148) + 0x10))();
    if (puStack_2b0 == (undefined1 *)0x0) {
      iVar5 = (**(code **)(*DAT_01ea573c + 0x18))();
      if (iVar5 == 0x2733) {
        ExceptionList = pvStack_c;
        return;
      }
      if (iVar5 != 0x2746) {
        _DAT_01dae774 = 0;
        ExceptionList = pvStack_c;
        return;
      }
    }
    piVar2 = *(int **)((int)this + 0x50);
    piVar7 = (int *)0x0;
    if ((((piVar2 != (int *)0x0) && (piVar2[0x13d7] == local_2a8._4_4_)) &&
        (*(short *)((int)piVar2 + 0x4f5a) == local_2a8._2_2_)) &&
       ((short)piVar2[0x13d6] == (short)local_2a8)) {
      piVar7 = piVar2;
    }
    iVar5 = 0;
    if (0 < *(int *)((int)this + 0x48)) {
      do {
        if (piVar7 != (int *)0x0) break;
        piVar3 = *(int **)(*(int *)((int)this + 0x44) + iVar5 * 4);
        if (((piVar3[0x13d7] == local_2a8._4_4_) &&
            (*(short *)((int)piVar3 + 0x4f5a) == local_2a8._2_2_)) &&
           ((short)piVar3[0x13d6] == (short)local_2a8)) {
          piVar7 = piVar3;
        }
        iVar5 = iVar5 + 1;
      } while (iVar5 < *(int *)((int)this + 0x48));
    }
    if (puStack_2b0 != (undefined1 *)0x0) {
      if (piVar7 == (int *)0x0) break;
      goto LAB_0047ef65;
    }
    if (((piVar7 != (int *)0x0) && (piVar7 != piVar2)) &&
       ((piVar7[0x1a] != 3 || (*(int *)((int)this + 0x130) == 0)))) {
      (**(code **)(*piVar7 + 0x120))();
    }
  } while( true );
  iVar5 = (**(code **)**(undefined4 **)((int)this + 0x54))();
  if (iVar5 == 1) {
    puStack_2b0 = &stack0xfffffd24;
    piVar7 = (int *)T_StaticClass_4(*(undefined4 **)((int)this + 0x108),-1,0,0,0,0,0,0,0);
    UGameEngine__unknown_00562310(auStack_290,(short *)0x0);
    uStack_4 = 0;
    (**(code **)(*piVar7 + 300))(this);
    uStack_24 = 0xffffffff;
    UGameEngine__unknown_0041bec0((int *)&puStack_2b0);
    (**(code **)(**(int **)((int)this + 0x54) + 4))(piVar7);
    iVar4 = *(int *)((int)this + 0x48);
    iVar5 = iVar4 + 1;
    *(int *)((int)this + 0x48) = iVar5;
    if (*(int *)((int)this + 0x4c) < iVar5) {
      iVar5 = ((int)(iVar5 * 3 + (iVar5 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar5;
      *(int *)((int)this + 0x4c) = iVar5;
      if ((*(int *)((int)this + 0x44) != 0) || (iVar5 != 0)) {
        puStack_2b0 = (undefined1 *)(iVar5 * 4);
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        uVar6 = (**(code **)(*DAT_01ea5778 + 8))();
        *(undefined4 *)((int)this + 0x44) = uVar6;
      }
    }
    puVar1 = (undefined4 *)(*(int *)((int)this + 0x44) + iVar4 * 4);
    if (puVar1 != (undefined4 *)0x0) {
      *puVar1 = piVar7;
    }
LAB_0047ef65:
    (**(code **)(*piVar7 + 0x14c))();
  }
  goto LAB_0047ed58;
}


/* [VTable] UTcpNetDriver virtual function #2
   VTable: vtable_UTcpNetDriver at 018012fc */

int * __thiscall UTcpNetDriver__vfunc_2(void *this,byte param_1)

{
  FUN_00480240(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UTcpipConnection.c ========== */

/*
 * SGW.exe - UTcpipConnection (6 functions)
 */

/* [VTable] UTcpipConnection virtual function #77
   VTable: vtable_UTcpipConnection at 01801034 */

void __thiscall UTcpipConnection__vfunc_77(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  int iVar2;
  u_long uVar3;
  undefined4 local_1c;
  LARGE_INTEGER local_18;
  undefined8 local_10;
  undefined8 local_8;
  
  iVar2 = *(int *)((int)this + 0x4f70);
  if (iVar2 != 0) {
    if (*(int *)(iVar2 + 0x31c) == 0) {
      return;
    }
    if (*(int *)(iVar2 + 0x114) != 0) {
      *(undefined4 *)(*(int *)(*(int *)((int)this + 100) + 0x50) + 0x68) = 1;
      if (*(undefined4 **)((int)this + 0x4f70) != (undefined4 *)0x0) {
        (**(code **)**(undefined4 **)((int)this + 0x4f70))(1);
      }
      *(undefined4 *)((int)this + 0x4f70) = 0;
      return;
    }
    local_10 = *(undefined8 *)(iVar2 + 4);
    local_8 = *(undefined8 *)(iVar2 + 0xc);
    FUN_00480df0(&local_18,(int)&local_10);
    uVar3 = htonl(local_18.s.LowPart);
    *(u_long *)((int)this + 0x4f5c) = uVar3;
    if (*(undefined4 **)((int)this + 0x4f70) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)((int)this + 0x4f70))(1);
    }
    *(undefined4 *)((int)this + 0x4f70) = 0;
  }
  local_1c = 0;
  QueryPerformanceCounter(&local_18);
  piVar1 = (int *)(*(int *)((int)this + 100) + 0x9c);
  *piVar1 = *piVar1 - local_18._0_4_;
  (**(code **)(**(int **)((int)this + 0x4f68) + 0xc))(param_1,param_2,&local_1c,(int)this + 0x4f58);
  return;
}


/* Also in vtable: UTcpipConnection (slot 0) */

undefined4 UTcpipConnection__vfunc_0(void)

{
  return 1;
}


/* [VTable] UTcpipConnection virtual function #1
   VTable: vtable_UTcpipConnection at 01801034 */

bool __fastcall UTcpipConnection__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2bc0 == (undefined4 *)0x0) {
    DAT_01ee2bc0 = FUN_005e0da0();
    FUN_005de540();
  }
  return puVar1 != DAT_01ee2bc0;
}


/* [VTable] UTcpipConnection virtual function #67
   VTable: vtable_UTcpipConnection at 01801034 */

void * __thiscall UTcpipConnection__vfunc_67(void *this,void *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167ec39;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  UTcpipConnection__unknown_0047f7a0((void *)((int)this + 0x4f58),param_1,1);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] UTcpipConnection virtual function #68
   VTable: vtable_UTcpipConnection at 01801034 */

void * __thiscall UTcpipConnection__vfunc_68(void *this,void *param_1)

{
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167ec64;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  UTcpipConnection__unknown_0047f7a0((void *)((int)this + 0x4f58),local_18,1);
  local_4 = 1;
  FUN_00efa4e0(param_1);
  local_4 = local_4 & 0xffffff00;
  FUN_0041b420(local_18);
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] UTcpipConnection virtual function #2
   VTable: vtable_UTcpipConnection at 01801034 */

void * __thiscall UTcpipConnection__vfunc_2(void *this,byte param_1)

{
  FUN_0047fff0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UserDataType.c ========== */

/*
 * SGW.exe - UserDataType (1 functions)
 */

undefined4 * __thiscall UserDataType__vfunc_0(void *this,byte param_1)

{
  FUN_01599000(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== UserMessage.c ========== */

/*
 * SGW.exe - UserMessage (1 functions)
 */

undefined4 * __thiscall UserMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01588120(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== UserMetaDataType.c ========== */

/*
 * SGW.exe - UserMetaDataType (1 functions)
 */

undefined4 * __thiscall UserMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_01598380(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== WholeMachineMessage.c ========== */

/*
 * SGW.exe - WholeMachineMessage (1 functions)
 */

undefined4 * __thiscall WholeMachineMessage__vfunc_0(void *this,byte param_1)

{
  FUN_01587de0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== WinFileSystem.c ========== */

/*
 * SGW.exe - WinFileSystem (11 functions)
 */

/* [VTable] WinFileSystem virtual function #6
   VTable: vtable_WinFileSystem at 017fd438 */

uint __thiscall
WinFileSystem__vfunc_6(void *this,undefined4 param_1,undefined4 param_2,char param_3)

{
  undefined *puVar1;
  FILE *_File;
  size_t sVar2;
  uint uVar3;
  undefined4 uVar4;
  int unaff_retaddr;
  void *pvStack_14;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167d568;
  pvStack_c = ExceptionList;
  local_4 = 0;
  puVar1 = &DAT_017fd3f8;
  if (param_3 == '\0') {
    puVar1 = &DAT_017fd3fc;
  }
  ExceptionList = &pvStack_c;
  _File = (FILE *)(**(code **)(*(int *)this + 0x24))(param_1,puVar1);
  if (_File != (FILE *)0x0) {
    if ((*(size_t *)(unaff_retaddr + 0xc) == 0) ||
       (sVar2 = fwrite(*(void **)(unaff_retaddr + 8),*(size_t *)(unaff_retaddr + 0xc),1,_File),
       sVar2 == 1)) {
      fclose(_File);
      pvStack_c = (void *)0xffffffff;
      uVar4 = FUN_004585a0((int *)&stack0x00000000);
      ExceptionList = pvStack_14;
      return CONCAT31((int3)((uint)uVar4 >> 8),1);
    }
    fclose(_File);
    Mercury__unknown_00a351d0
              ((int *)&stack0xffffffe4,"WinFileSystem: Error writing to %s. Disk full?\n");
  }
  pvStack_c = (void *)0xffffffff;
  uVar3 = FUN_004585a0((int *)&stack0x00000000);
  ExceptionList = pvStack_14;
  return uVar3 & 0xffffff00;
}


/* Also in vtable: WinFileSystem (slot 0) */

undefined4 * __thiscall WinFileSystem__vfunc_0(void *this,byte param_1)

{
  FUN_00470e80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] WinFileSystem virtual function #10
   VTable: vtable_WinFileSystem at 017fd438 */

void __fastcall WinFileSystem__vfunc_10(int param_1)

{
  void *this;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169047b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this = (void *)scalable_malloc(0x20);
  if (this == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00471030(this,(void *)(param_1 + 4));
  ExceptionList = local_c;
  return;
}


/* [VTable] WinFileSystem virtual function #1
   VTable: vtable_WinFileSystem at 017fd438
   [String discovery] References: "WinFileSystem::getFileType" */

int __thiscall WinFileSystem__vfunc_1(void *this,uint param_1,undefined4 *param_2)

{
  uint uVar1;
  uint uVar2;
  void *pvVar3;
  LPCSTR lpFileName;
  BOOL BVar4;
  undefined4 *puVar5;
  char local_6d;
  undefined4 *puStack_6c;
  undefined4 uStack_68;
  undefined4 local_64 [4];
  undefined4 local_54;
  uint local_50;
  undefined1 auStack_4c [4];
  undefined4 auStack_48 [4];
  undefined4 uStack_38;
  uint uStack_34;
  uint uStack_30;
  undefined4 uStack_2c;
  undefined4 uStack_28;
  undefined4 uStack_24;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  undefined4 uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d598;
  pvStack_c = ExceptionList;
  local_50 = 0xf;
  local_6d = '\0';
  local_54 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_64,&local_6d);
  uVar1 = std::char_traits<char>::length("WinFileSystem::getFileType");
  FUN_004377d0(&uStack_68,"WinFileSystem::getFileType",uVar1);
  uVar1 = param_1;
  uStack_4 = 0;
  WinFileSystem__unknown_00458290(param_1);
  uStack_4 = 0xffffffff;
  if (0xf < local_50) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_64[0]);
  }
  local_50 = 0xf;
  param_1 = param_1 & 0xffffff00;
  local_54 = 0;
  std::char_traits<char>::assign((char *)local_64,(char *)&param_1);
  if (*(int *)(uVar1 + 0x14) != 0) {
    local_50 = 0xf;
    param_1 = param_1 & 0xffffff00;
    local_54 = 0;
    std::char_traits<char>::assign((char *)local_64,(char *)&param_1);
    uVar2 = std::char_traits<char>::length("ft");
    FUN_004377d0(&uStack_68,"ft",uVar2);
    uStack_4 = 1;
    puVar5 = &uStack_68;
    pvVar3 = (void *)WinFileSystem__unknown_00a36340();
    puStack_6c = WinFileSystem__unknown_00a360b0(pvVar3,puVar5);
    uStack_4 = 0xffffffff;
    if (0xf < local_50) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_64[0]);
    }
    local_50 = 0xf;
    param_1 = param_1 & 0xffffff00;
    local_54 = 0;
    std::char_traits<char>::assign((char *)local_64,(char *)&param_1);
    pvVar3 = WinFileSystem__unknown_00458a50(auStack_4c,(void *)((int)this + 4),uVar1);
    uStack_4 = 2;
    if (*(uint *)((int)pvVar3 + 0x18) < 0x10) {
      lpFileName = (LPCSTR)((int)pvVar3 + 4);
    }
    else {
      lpFileName = *(LPCSTR *)((int)pvVar3 + 4);
    }
    BVar4 = GetFileAttributesExA(lpFileName,GetFileExInfoStandard,&uStack_30);
    uStack_4 = 0xffffffff;
    if (0xf < uStack_34) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_48[0]);
    }
    uStack_34 = 0xf;
    param_1 = param_1 & 0xffffff00;
    uStack_38 = 0;
    std::char_traits<char>::assign((char *)auStack_48,(char *)&param_1);
    WinFileSystem__unknown_00470e70((int)puStack_6c);
    if ((BVar4 != 0) && (uStack_30 != 0xffffffff)) {
      if (param_2 != (undefined4 *)0x0) {
        *param_2 = uStack_10;
        param_2[2] = uStack_2c;
        param_2[3] = uStack_28;
        param_2[4] = uStack_1c;
        param_2[5] = uStack_18;
        param_2[6] = uStack_24;
        param_2[1] = uStack_14;
        param_2[7] = uStack_20;
      }
      ExceptionList = pvStack_c;
      return 2 - (uint)((uStack_30 & 0x10) != 0);
    }
  }
  ExceptionList = pvStack_c;
  return 0;
}


/* [VTable] WinFileSystem virtual function #2
   VTable: vtable_WinFileSystem at 017fd438 */

void * WinFileSystem__vfunc_2(void *param_1)

{
  uint uVar1;
  char acStack_41 [5];
  _SYSTEMTIME local_3c;
  char acStack_2c [32];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d5c9;
  pvStack_c = ExceptionList;
  acStack_41[1] = '\0';
  acStack_41[2] = '\0';
  acStack_41[3] = '\0';
  acStack_41[4] = '\0';
  local_3c.wYear = 0;
  local_3c.wMonth = 0;
  local_3c.wDayOfWeek = 0;
  local_3c.wDay = 0;
  local_3c.wHour = 0;
  local_3c.wMinute = 0;
  local_3c.wSecond = 0;
  local_3c.wMilliseconds = 0;
  ExceptionList = &pvStack_c;
  FileTimeToSystemTime((FILETIME *)&stack0x00000008,&local_3c);
  sprintf(acStack_2c,"%s %s %02d %02d:%02d:%02d %d",(&PTR_PTR_01dae120)[local_3c.wDayOfWeek],
          (&PTR_PTR_01dae13c)[local_3c.wMonth],(uint)local_3c.wDay,(uint)local_3c.wHour,
          (uint)local_3c.wMinute,(uint)local_3c.wSecond,(uint)local_3c.wYear);
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  acStack_41[0] = '\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<char>::assign((char *)((int)param_1 + 4),acStack_41);
  uVar1 = std::char_traits<char>::length(acStack_2c);
  FUN_004377d0(param_1,acStack_2c,uVar1);
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] WinFileSystem virtual function #4
   VTable: vtable_WinFileSystem at 017fd438
   [String discovery] References: "WinFileSystem::readFile" */

undefined4 * __thiscall WinFileSystem__vfunc_4(void *this,undefined4 param_1,int param_2)

{
  uint uVar1;
  void *pvVar2;
  FILE *_File;
  size_t sVar3;
  undefined4 **ppuVar4;
  size_t sVar5;
  undefined4 *puVar6;
  int iVar7;
  undefined4 uStack_70;
  size_t local_6c;
  undefined4 *puStack_68;
  undefined4 *puStack_64;
  undefined1 *puStack_60;
  undefined4 *puStack_5c;
  exception aeStack_58 [20];
  undefined1 auStack_44 [4];
  char local_40 [16];
  undefined4 local_30;
  uint local_2c;
  undefined4 uStack_28;
  char acStack_24 [16];
  void *pvStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 *puStack_4;
  
  puStack_4 = (undefined4 *)0xffffffff;
  puStack_8 = &LAB_0167d65c;
  pvStack_c = ExceptionList;
  iVar7 = 0;
  local_6c = 0;
  local_2c = 0xf;
  uStack_70 = (undefined4 *)(uint)(uint3)uStack_70;
  local_30 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_40,(char *)((int)&uStack_70 + 3));
  uVar1 = std::char_traits<char>::length("WinFileSystem::readFile");
  FUN_004377d0(auStack_44,"WinFileSystem::readFile",uVar1);
  puStack_4 = (undefined4 *)0x1;
  WinFileSystem__unknown_00458290(param_2);
  puStack_4 = (undefined4 *)((uint)puStack_4 & 0xffffff00);
  if (0xf < local_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  local_2c = 0xf;
  uStack_70 = (undefined4 *)((uint)uStack_70 & 0xffffff);
  local_30 = 0;
  std::char_traits<char>::assign(local_40,(char *)((int)&uStack_70 + 3));
  uStack_10 = 0xf;
  uStack_70 = (undefined4 *)((uint)uStack_70 & 0xffffff);
  pvStack_14 = (void *)0x0;
  std::char_traits<char>::assign(acStack_24,(char *)((int)&uStack_70 + 3));
  uVar1 = std::char_traits<char>::length("fo");
  FUN_004377d0(&uStack_28,"fo",uVar1);
  puStack_4 = (undefined4 *)0x2;
  puVar6 = &uStack_28;
  pvVar2 = (void *)WinFileSystem__unknown_00a36340();
  puStack_64 = WinFileSystem__unknown_00a360b0(pvVar2,puVar6);
  puStack_4 = (undefined4 *)((uint)puStack_4 & 0xffffff00);
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_10 = 0xf;
  uStack_70 = (undefined4 *)((uint)uStack_70 & 0xffffff);
  pvStack_14 = (void *)0x0;
  std::char_traits<char>::assign(acStack_24,(char *)((int)&uStack_70 + 3));
  _File = (FILE *)(**(code **)(*(int *)this + 0x24))();
  WinFileSystem__unknown_00470e70(local_6c);
  if (_File == (FILE *)0x0) {
    *puStack_4 = 0;
    ExceptionList = pvStack_14;
    return puStack_4;
  }
  fseek(_File,0,2);
  sVar3 = ftell(_File);
  local_6c = sVar3;
  fseek(_File,0,0);
  if ((int)sVar3 < 1) {
    _00___FRawStaticIndexBuffer__vfunc_0();
    fclose(_File);
    *puStack_4 = 0;
    ExceptionList = pvStack_14;
    return puStack_4;
  }
  puStack_5c = (undefined4 *)scalable_malloc();
  if (puStack_5c == (undefined4 *)0x0) {
    FUN_004099f0(aeStack_58);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(aeStack_58,(ThrowInfo *)&DAT_01d65cc8);
  }
  puStack_60 = &stack0xffffff74;
  puStack_64 = (undefined4 *)&stack0xffffff74;
  pvStack_c._1_3_ = 0;
  pvStack_c._0_1_ = 5;
  uStack_70 = FUN_00472290(puStack_5c,(void *)0x0,sVar3,0);
  pvStack_c = (void *)((uint)pvStack_c._1_3_ << 8);
  if (uStack_70 != (undefined4 *)0x0) {
    FUN_00457e40((int)uStack_70);
  }
  pvStack_c = (void *)0x9;
  puStack_60 = (undefined1 *)uStack_70[2];
  if (puStack_60 == (undefined1 *)0x0) {
    _00___FRawStaticIndexBuffer__vfunc_0();
    fclose(_File);
    *puStack_4 = 0;
    pvStack_c = (void *)((uint)pvStack_c & 0xffffff00);
    puVar6 = puStack_4;
  }
  else {
    BW__unknown_00438c40(&local_30,"fr");
    pvStack_c._0_1_ = 0xb;
    puVar6 = &local_30;
    pvVar2 = (void *)WinFileSystem__unknown_00a36340();
    puStack_5c = WinFileSystem__unknown_00a360b0(pvVar2,puVar6);
    pvStack_c = (void *)CONCAT31(pvStack_c._1_3_,9);
    ZipFileSystem__unknown_00433ed0((uint)&local_30);
    sVar5 = 1;
    if (0 < (int)sVar3) {
      puStack_68 = (undefined4 *)0x20000;
      do {
        puStack_64 = (undefined4 *)(sVar3 - iVar7);
        ppuVar4 = &puStack_68;
        if ((int)(sVar3 - iVar7) < 0x20001) {
          ppuVar4 = &puStack_64;
        }
        puVar6 = *ppuVar4;
        sVar5 = fread((void *)(iVar7 + (int)puStack_60),(size_t)puVar6,1,_File);
      } while ((sVar5 != 0) &&
              (iVar7 = iVar7 + (int)puVar6, sVar3 = local_6c, iVar7 < (int)local_6c));
    }
    WinFileSystem__unknown_00470e70((int)puStack_5c);
    if (sVar5 == 0) {
      _00___FRawStaticIndexBuffer__vfunc_0();
      fclose(_File);
      *puStack_4 = 0;
      pvStack_c = (void *)((uint)pvStack_c & 0xffffff00);
      puVar6 = puStack_4;
    }
    else {
      fclose(_File);
      puVar6 = puStack_4;
      FUN_00458600(puStack_4,&uStack_70);
    }
  }
  pvStack_c = (void *)((uint)pvStack_c & 0xffffff00);
  FUN_004585a0(&uStack_70);
  ExceptionList = pvStack_14;
  return puVar6;
}


/* [VTable] WinFileSystem virtual function #5
   VTable: vtable_WinFileSystem at 017fd438 */

bool __thiscall WinFileSystem__vfunc_5(void *this,uint param_1)

{
  void *pvVar1;
  LPCSTR lpPathName;
  BOOL BVar2;
  undefined1 local_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d5d28;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  pvVar1 = WinFileSystem__unknown_00458a50(local_28,(void *)((int)this + 4),param_1);
  local_4 = 0;
  if (*(uint *)((int)pvVar1 + 0x18) < 0x10) {
    lpPathName = (LPCSTR)((int)pvVar1 + 4);
  }
  else {
    lpPathName = *(LPCSTR *)((int)pvVar1 + 4);
  }
  BVar2 = CreateDirectoryA(lpPathName,(LPSECURITY_ATTRIBUTES)0x0);
  local_4 = 0xffffffff;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_1 = param_1 & 0xffffff00;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  ExceptionList = pvStack_c;
  return BVar2 == 1;
}


/* [VTable] WinFileSystem virtual function #9
   VTable: vtable_WinFileSystem at 017fd438 */

FILE * __thiscall WinFileSystem__vfunc_9(void *this,int param_1,char *param_2)

{
  char ***_Filename;
  FILE *pFVar1;
  undefined1 local_28 [4];
  char **local_24 [4];
  undefined4 uStack_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d5d28;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  WinFileSystem__unknown_00458a50(local_28,(void *)((int)this + 4),param_1);
  local_4 = 0;
  _Filename = (char ***)local_24[0];
  if (local_10 < 0x10) {
    _Filename = local_24;
  }
  pFVar1 = fopen((char *)_Filename,param_2);
  if (pFVar1 == (FILE *)0x0) {
    local_4 = 0xffffffff;
    if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_24[0]);
    }
    local_10 = 0xf;
    param_1 = (uint)param_1._1_3_ << 8;
    uStack_14 = 0;
    std::char_traits<char>::assign((char *)local_24,(char *)&param_1);
    ExceptionList = pvStack_c;
    return (FILE *)0x0;
  }
  local_4 = 0xffffffff;
  if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 0xf;
  param_1 = (uint)param_1._1_3_ << 8;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)local_24,(char *)&param_1);
  ExceptionList = pvStack_c;
  return pFVar1;
}


/* [VTable] WinFileSystem virtual function #7
   VTable: vtable_WinFileSystem at 017fd438 */

bool __thiscall WinFileSystem__vfunc_7(void *this,int param_1,uint param_2)

{
  void *pvVar1;
  void *pvVar2;
  LPCSTR lpExistingFileName;
  BOOL BVar3;
  LPCSTR lpNewFileName;
  undefined1 local_44 [4];
  undefined4 auStack_40 [4];
  undefined4 uStack_30;
  uint uStack_2c;
  undefined1 local_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01796090;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  pvVar1 = WinFileSystem__unknown_00458a50(local_28,(void *)((int)this + 4),param_2);
  local_4 = 0;
  pvVar2 = WinFileSystem__unknown_00458a50(local_44,(void *)((int)this + 4),param_1);
  local_4._0_1_ = 1;
  if (*(uint *)((int)pvVar1 + 0x18) < 0x10) {
    lpNewFileName = (LPCSTR)((int)pvVar1 + 4);
  }
  else {
    lpNewFileName = *(LPCSTR *)((int)pvVar1 + 4);
  }
  if (*(uint *)((int)pvVar2 + 0x18) < 0x10) {
    lpExistingFileName = (LPCSTR)((int)pvVar2 + 4);
  }
  else {
    lpExistingFileName = *(LPCSTR *)((int)pvVar2 + 4);
  }
  BVar3 = MoveFileExA(lpExistingFileName,lpNewFileName,1);
  local_4 = (uint)local_4._1_3_ << 8;
  if (0xf < uStack_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_40[0]);
  }
  uStack_2c = 0xf;
  param_2 = param_2 & 0xffffff00;
  uStack_30 = 0;
  std::char_traits<char>::assign((char *)auStack_40,(char *)&param_2);
  local_4 = 0xffffffff;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_2 = param_2 & 0xffffff00;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_2);
  ExceptionList = pvStack_c;
  return BVar3 != 0;
}


/* [VTable] WinFileSystem virtual function #8
   VTable: vtable_WinFileSystem at 017fd438 */

bool __thiscall WinFileSystem__vfunc_8(void *this,void *param_1)

{
  char cVar1;
  int iVar2;
  void *pvVar3;
  LPCSTR pCVar4;
  BOOL BVar5;
  uint uVar6;
  undefined1 *puVar7;
  uint uStack_6c;
  undefined1 auStack_68 [4];
  uint uStack_64;
  uint uStack_60;
  undefined4 uStack_54;
  uint uStack_50;
  undefined1 auStack_4c [4];
  undefined4 auStack_48 [4];
  undefined4 uStack_38;
  uint uStack_34;
  undefined1 auStack_30 [4];
  undefined4 auStack_2c [4];
  undefined4 uStack_1c;
  uint uStack_18;
  void *pvStack_14;
  void *pvStack_c;
  undefined1 *puStack_8;
  void *pvStack_4;
  
  pvStack_4 = (void *)0xffffffff;
  puStack_8 = &LAB_0167d698;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar2 = (**(code **)(*(int *)this + 4))(param_1,0);
  if (iVar2 != 2) {
    if (iVar2 == 1) {
      (**(code **)(*(int *)this + 0xc))(auStack_68,param_1);
      pvStack_c = (void *)0x1;
      uStack_6c = 0;
      iVar2 = 0;
      while( true ) {
        if ((uStack_64 == 0) || ((uint)((int)(uStack_60 - uStack_64) / 0x1c) <= uStack_6c)) {
          pvVar3 = WinFileSystem__unknown_00458a50(auStack_30,(void *)((int)this + 4),(int)param_1);
          pvStack_c._0_1_ = 4;
          if (*(uint *)((int)pvVar3 + 0x18) < 0x10) {
            pCVar4 = (LPCSTR)((int)pvVar3 + 4);
          }
          else {
            pCVar4 = *(LPCSTR *)((int)pvVar3 + 4);
          }
          BVar5 = RemoveDirectoryA(pCVar4);
          pvStack_c = (void *)CONCAT31(pvStack_c._1_3_,1);
          if (uStack_18 < 0x10) {
            uStack_18 = 0xf;
            pvStack_4 = (void *)((uint)pvStack_4 & 0xffffff00);
            uStack_1c = 0;
            std::char_traits<char>::assign((char *)auStack_2c,(char *)&pvStack_4);
            pvStack_c = (void *)0xffffffff;
            if (uStack_64 == 0) {
              ExceptionList = pvStack_14;
              return BVar5 != 0;
            }
            puVar7 = auStack_68;
            uVar6 = uStack_64;
            pvVar3 = pvStack_4;
            WinFileSystem__unknown_00459570(uStack_64,uStack_60);
                    /* WARNING: Subroutine does not return */
            scalable_free(uStack_64,uVar6,uStack_60,puVar7,pvVar3);
          }
                    /* WARNING: Subroutine does not return */
          scalable_free(auStack_2c[0]);
        }
        pvVar3 = BWResource__unknown_00459470(auStack_30,param_1,"/");
        pvStack_c._0_1_ = 2;
        if ((uStack_64 == 0) || ((uint)((int)(uStack_60 - uStack_64) / 0x1c) <= uStack_6c)) {
          _invalid_parameter_noinfo();
        }
        pvVar3 = WinFileSystem__unknown_00458a50(auStack_4c,pvVar3,iVar2 + uStack_64);
        pvStack_c._0_1_ = 3;
        cVar1 = (**(code **)(*(int *)this + 0x20))(pvVar3);
        pvStack_c._0_1_ = 2;
        if (0xf < uStack_34) {
                    /* WARNING: Subroutine does not return */
          scalable_free(auStack_48[0]);
        }
        uStack_34 = 0xf;
        uStack_38 = 0;
        std::char_traits<char>::assign((char *)auStack_48,&stack0xffffff8f);
        pvStack_c = (void *)CONCAT31(pvStack_c._1_3_,1);
        if (0xf < uStack_18) {
                    /* WARNING: Subroutine does not return */
          scalable_free(auStack_2c[0]);
        }
        uStack_18 = 0xf;
        uStack_1c = 0;
        std::char_traits<char>::assign((char *)auStack_2c,&stack0xffffff8f);
        if (cVar1 == '\0') break;
        uStack_6c = uStack_6c + 1;
        iVar2 = iVar2 + 0x1c;
        param_1 = pvStack_4;
      }
      pvStack_c = (void *)0xffffffff;
      if (uStack_64 != 0) {
        puVar7 = auStack_68;
        uVar6 = uStack_64;
        pvVar3 = pvStack_4;
        WinFileSystem__unknown_00459570(uStack_64,uStack_60);
                    /* WARNING: Subroutine does not return */
        scalable_free(uStack_64,uVar6,uStack_60,puVar7,pvVar3);
      }
    }
    ExceptionList = pvStack_14;
    return false;
  }
  pvVar3 = WinFileSystem__unknown_00458a50(auStack_68,(void *)((int)this + 4),(int)param_1);
  pvStack_c = (void *)0x0;
  if (*(uint *)((int)pvVar3 + 0x18) < 0x10) {
    pCVar4 = (LPCSTR)((int)pvVar3 + 4);
  }
  else {
    pCVar4 = *(LPCSTR *)((int)pvVar3 + 4);
  }
  BVar5 = DeleteFileA(pCVar4);
  pvStack_c = (void *)0xffffffff;
  if (uStack_50 < 0x10) {
    uStack_50 = 0xf;
    pvStack_4 = (void *)((uint)pvStack_4 & 0xffffff00);
    uStack_54 = 0;
    std::char_traits<char>::assign((char *)&uStack_64,(char *)&pvStack_4);
    ExceptionList = pvStack_14;
    return BVar5 != 0;
  }
                    /* WARNING: Subroutine does not return */
  scalable_free(uStack_64);
}


/* [VTable] WinFileSystem virtual function #3
   VTable: vtable_WinFileSystem at 017fd438
   [String discovery] References: "WinFileSystem::readDirectory" */

void * __thiscall WinFileSystem__vfunc_3(void *this,void *param_1,int param_2)

{
  uint uVar1;
  int iVar2;
  char *pcVar3;
  LPCSTR ****lpFileName;
  uint uVar4;
  uint uVar5;
  BOOL BVar6;
  char *****pppppcVar7;
  undefined4 *puVar8;
  undefined1 *puVar9;
  HANDLE pvVar10;
  char local_1aa [2];
  HANDLE pvStack_1a8;
  undefined1 auStack_1a4 [4];
  uint uStack_1a0;
  uint uStack_19c;
  undefined4 uStack_198;
  undefined1 auStack_194 [4];
  char ****local_190 [4];
  uint local_180;
  uint local_17c;
  undefined4 local_178;
  undefined1 auStack_174 [4];
  LPCSTR ***appppCStack_170 [4];
  int iStack_160;
  uint uStack_15c;
  undefined4 auStack_158 [3];
  _WIN32_FIND_DATAA _Stack_14c;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d6fb;
  pvStack_c = ExceptionList;
  local_178 = 0;
  local_17c = 0xf;
  local_1aa[0] = '\0';
  local_180 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_190,local_1aa);
  uVar1 = std::char_traits<char>::length("WinFileSystem::readDirectory");
  FUN_004377d0(auStack_194,"WinFileSystem::readDirectory",uVar1);
  uStack_4 = 1;
  WinFileSystem__unknown_00458290(param_2);
  uStack_4 = uStack_4 & 0xffffff00;
  if (0xf < local_17c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_190[0]);
  }
  local_17c = 0xf;
  local_1aa[0] = '\0';
  local_180 = 0;
  std::char_traits<char>::assign((char *)local_190,local_1aa);
  uStack_1a0 = 0;
  uStack_19c = 0;
  uStack_198 = 0;
  uStack_4 = 2;
  local_1aa[0] = '\0';
  WinFileSystem__unknown_00458a50(auStack_174,(void *)((int)this + 4),param_2);
  uStack_4._0_1_ = 3;
  if (iStack_160 != 0) {
    iVar2 = FUN_00458420(auStack_174,auStack_158);
    pcVar3 = (char *)FUN_004584b0(iVar2);
    if (*pcVar3 != '\\') {
      iVar2 = FUN_00458420(auStack_174,auStack_158);
      pcVar3 = (char *)FUN_004584b0(iVar2);
      if (*pcVar3 != '/') {
        uVar1 = std::char_traits<char>::length("\\*");
        puVar8 = (undefined4 *)&DAT_017fd350;
        goto LAB_00472025;
      }
    }
  }
  uVar1 = std::char_traits<char>::length("*");
  puVar8 = (undefined4 *)&DAT_017fd354;
LAB_00472025:
  BWResource__unknown_004588e0(auStack_174,puVar8,uVar1);
  lpFileName = (LPCSTR ****)appppCStack_170[0];
  if (uStack_15c < 0x10) {
    lpFileName = appppCStack_170;
  }
  pvStack_1a8 = FindFirstFileA((LPCSTR)lpFileName,&_Stack_14c);
  if (pvStack_1a8 == (HANDLE)0xffffffff) {
    FUN_00462910(param_1,auStack_1a4);
    local_178 = 1;
    uStack_4._0_1_ = 2;
    if (0xf < uStack_15c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(appppCStack_170[0]);
    }
    pcVar3 = local_1aa;
    local_1aa[0] = '\0';
  }
  else {
    do {
      local_17c = 0xf;
      local_1aa[1] = 0;
      local_180 = 0;
      std::char_traits<char>::assign((char *)local_190,local_1aa + 1);
      uVar1 = std::char_traits<char>::length(_Stack_14c.cFileName);
      FUN_004377d0(auStack_194,_Stack_14c.cFileName,uVar1);
      uStack_4 = CONCAT31(uStack_4._1_3_,4);
      uVar4 = std::char_traits<char>::length(".");
      uVar1 = local_180;
      uVar5 = local_180;
      if (uVar4 <= local_180) {
        uVar5 = uVar4;
      }
      pppppcVar7 = (char *****)local_190[0];
      if (local_17c < 0x10) {
        pppppcVar7 = local_190;
      }
      iVar2 = std::char_traits<char>::compare((char *)pppppcVar7,".",uVar5);
      if (((iVar2 != 0) || (uVar1 < uVar4)) || (uVar1 != uVar4)) {
        uVar1 = std::char_traits<char>::length("..");
        uVar1 = FUN_004242c0(auStack_194,0,local_180,"..",uVar1);
        if (uVar1 != 0) {
          FUN_0045b4b0(auStack_1a4,auStack_194);
        }
      }
      BVar6 = FindNextFileA(pvStack_1a8,&_Stack_14c);
      if (BVar6 == 0) {
        local_1aa[0] = '\x01';
      }
      uStack_4._0_1_ = 3;
      if (0xf < local_17c) {
                    /* WARNING: Subroutine does not return */
        scalable_free(local_190[0]);
      }
      local_17c = 0xf;
      local_1aa[1] = 0;
      local_180 = 0;
      std::char_traits<char>::assign((char *)local_190,local_1aa + 1);
    } while (local_1aa[0] == '\0');
    FindClose(pvStack_1a8);
    FUN_00462910(param_1,auStack_1a4);
    local_178 = 1;
    uStack_4._0_1_ = 2;
    if (0xf < uStack_15c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(appppCStack_170[0]);
    }
    pcVar3 = local_1aa + 1;
    local_1aa[1] = 0;
  }
  uStack_4._0_1_ = 2;
  local_178 = 1;
  uStack_15c = 0xf;
  iStack_160 = 0;
  std::char_traits<char>::assign((char *)appppCStack_170,pcVar3);
  uStack_4 = (uint)uStack_4._1_3_ << 8;
  if (uStack_1a0 != 0) {
    puVar9 = auStack_1a4;
    uVar1 = uStack_1a0;
    uVar5 = uStack_19c;
    pvVar10 = pvStack_1a8;
    WinFileSystem__unknown_00459570(uStack_1a0,uStack_19c);
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_1a0,uVar1,uVar5,puVar9,pvVar10);
  }
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== ZipArchiveLib_CDeflateCompressor.c ========== */

/*
 * SGW.exe - ZipArchiveLib_CDeflateCompressor (1 functions)
 */

undefined4 ZipArchiveLib_CDeflateCompressor__vfunc_0(short param_1)

{
  if ((param_1 != 0) && (param_1 != 8)) {
    return 0;
  }
  return 1;
}




/* === Standalone functions (42) === */

/* Mercury debug (Channel): UChannel::IsKnownChannelType(ChType) */

int * __thiscall Mercury_Channel_7(void *this,uint param_1,undefined4 param_2,uint param_3)

{
  undefined4 *puVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  int *piVar5;
  undefined4 uVar6;
  
  iVar4 = FUN_008c7400(param_1);
  if (iVar4 == 0) {
    FUN_00486000("UChannel::IsKnownChannelType(ChType)",".\\Src\\UnConn.cpp",0x3fd);
  }
  (**(code **)(*(int *)this + 0x13c))();
  if (param_3 == 0xffffffff) {
    param_3 = (uint)(param_1 != 1);
    if (param_3 < 0x3ff) {
      piVar5 = (int *)((int)this + param_3 * 4 + 0xeb0);
      do {
        if (*piVar5 == 0) break;
        param_3 = param_3 + 1;
        piVar5 = piVar5 + 1;
      } while ((int)param_3 < 0x3ff);
    }
    if (param_3 == 0x3ff) {
      return (int *)0x0;
    }
  }
  if (0x3fe < (int)param_3) {
    FUN_00486000("ChIndex<MAX_CHANNELS",".\\Src\\UnConn.cpp",0x40e);
  }
  if (*(int *)((int)this + param_3 * 4 + 0xeb0) != 0) {
    FUN_00486000("Channels[ChIndex]==NULL",".\\Src\\UnConn.cpp",0x40f);
  }
  piVar5 = (int *)T_StaticClass_18(*(undefined4 **)(&DAT_01ee64d0 + param_1 * 4),-1,0,0,0,0,0,0,0);
  (**(code **)(*piVar5 + 0x10c))(this,param_3,param_2);
  *(int **)((int)this + param_3 * 4 + 0xeb0) = piVar5;
  iVar2 = *(int *)((int)this + 0x4ebc);
  iVar4 = iVar2 + 1;
  *(int *)((int)this + 0x4ebc) = iVar4;
  if (*(int *)((int)this + 0x4ec0) < iVar4) {
    iVar3 = *(int *)((int)this + 0x4eb8);
    iVar4 = ((int)(iVar4 * 3 + (iVar4 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar4;
    *(int *)((int)this + 0x4ec0) = iVar4;
    if ((iVar3 != 0) || (iVar4 != 0)) {
      if (DAT_01ea5778 == (int *)0x0) {
        FUN_00416660();
      }
      uVar6 = (**(code **)(*DAT_01ea5778 + 8))(iVar3,iVar4 * 4,8);
      *(undefined4 *)((int)this + 0x4eb8) = uVar6;
    }
  }
  puVar1 = (undefined4 *)(*(int *)((int)this + 0x4eb8) + iVar2 * 4);
  if (puVar1 != (undefined4 *)0x0) {
    *puVar1 = piVar5;
  }
  return piVar5;
}


/* Mercury error in Channel: #ferror in function 'getChannels'. */

undefined4 __cdecl Mercury_Channel(lua_State *param_1)

{
  int iVar1;
  uint local_c [3];
  
  iVar1 = CEGUI__unknown_00403280(param_1,1,local_c);
  if (iVar1 == 0) {
    CEGUI__unknown_00402f40(param_1,L"#ferror in function \'getChannels\'.",local_c);
    return 0;
  }
  iVar1 = FUN_00adc8c0();
  lua_pushvalue(param_1,iVar1);
  return 1;
}


/* Mercury debug (Channel): SoundNodeWave->ChannelOffsets.Num() == 0 */

undefined4 __cdecl Mercury_Channel_8(int param_1,int *param_2,int *param_3)

{
  int iVar1;
  
  if (param_2 == (int *)0x0) {
    FUN_00486000("SoundCooker",".\\Src\\UnEditor.cpp",0x726);
  }
  iVar1 = FUN_005066a0(param_3);
  if ((0 < iVar1) && (0 < *(int *)(param_1 + 0x70))) {
    return 0;
  }
  if (*(int *)(param_1 + 0x8c) == 0) {
    if (*(int *)(param_1 + 0x80) != 0) {
      FUN_00486000("SoundNodeWave->ChannelOffsets.Num() == 0",".\\Src\\UnEditor.cpp",0x731);
    }
    if (*(int *)(param_1 + 0x8c) != 0) {
      FUN_00486000("SoundNodeWave->ChannelSizes.Num() == 0",".\\Src\\UnEditor.cpp",0x732);
    }
    FUN_00b21610(param_1,param_2,param_3);
    return 1;
  }
  if (0 < *(int *)(param_1 + 0x8c)) {
    if (*(int *)(param_1 + 0x80) != 8) {
      FUN_00486000("SoundNodeWave->ChannelOffsets.Num() == SPEAKER_Count",".\\Src\\UnEditor.cpp",
                   0x737);
    }
    if (*(int *)(param_1 + 0x8c) != 8) {
      FUN_00486000("SoundNodeWave->ChannelSizes.Num() == SPEAKER_Count",".\\Src\\UnEditor.cpp",0x738
                  );
    }
    FUN_00b21750(param_1,param_2,param_3);
  }
  return 1;
}


/* [String discovery] Debug string: "u"BWClassMap::InitEntityClassTable:  unable to load
   entities.xml!""
   String at: 019aa560 */

void BWClassMap_InitEntityClassTable(void)

{
  int iVar1;
  byte bVar2;
  uint uVar3;
  int *extraout_ECX;
  int *piVar4;
  char local_31;
  int iStack_30;
  undefined1 *puStack_2c;
  undefined1 auStack_28 [4];
  char local_24 [16];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016f46c3;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  local_31 = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,&local_31);
  uVar3 = std::char_traits<char>::length("entities/entities.xml");
  FUN_004377d0(auStack_28,"entities/entities.xml",uVar3);
  uStack_4 = 0;
  BWResource__unknown_0045dea0(&iStack_30,auStack_28);
  uStack_4._0_1_ = 2;
  if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  local_10 = 0xf;
  local_31 = '\0';
  local_14 = 0;
  std::char_traits<char>::assign(local_24,&local_31);
  if (iStack_30 != 0) {
    puStack_2c = &stack0xffffffc0;
    piVar4 = extraout_ECX;
    FUN_00458600(&stack0xffffffc0,&iStack_30);
    uStack_4 = CONCAT31(uStack_4._1_3_,3);
    if (DAT_01ef244c == 0) {
      FUN_00486000("NULL != instance_",".\\Src\\GameEntityManager.cpp",0x50);
    }
    iVar1 = DAT_01ef244c;
    if (DAT_01ef244c == 0) {
      FUN_00486000("ref",".\\Src\\GameEntityManager.cpp",0x53);
    }
    uStack_4._0_1_ = 2;
    bVar2 = EntityDescriptionMap_parse(*(void **)(iVar1 + 0x90),piVar4);
    if (bVar2 != 0) goto LAB_00c672c2;
  }
  FUN_00492470(0,L"BWClassMap::InitEntityClassTable:  unable to load entities.xml!");
  FUN_00486000("FALSE",".\\Src\\GameEntityManager.cpp",0x5f);
LAB_00c672c2:
  uStack_4 = 0xffffffff;
  FUN_004585a0(&iStack_30);
  ExceptionList = pvStack_c;
  return;
}


/* Mercury debug (Channel): aEvent->getProperty( "Channel", channel ) */

void Mercury_Channel_9(void *param_1)

{
  undefined4 *puVar1;
  bool bVar2;
  char cVar3;
  uint uVar4;
  void *this;
  int iVar5;
  undefined4 **ppuVar6;
  undefined1 *puVar7;
  undefined1 *puVar8;
  char cStack_8c;
  byte bStack_8b;
  char cStack_8a;
  byte bStack_89;
  wchar_t local_88 [2];
  undefined1 auStack_84 [4];
  undefined4 auStack_80 [4];
  undefined4 uStack_70;
  uint uStack_6c;
  undefined4 *puStack_68;
  void *local_64;
  undefined1 auStack_60 [4];
  undefined4 auStack_5c [4];
  undefined4 uStack_4c;
  uint uStack_48;
  undefined1 auStack_44 [4];
  undefined4 local_40 [4];
  undefined4 local_30;
  uint local_2c;
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_016f9543;
  pvStack_c = ExceptionList;
  local_2c = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  local_30 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<wchar_t>::assign((wchar_t *)local_40,local_88);
  iStack_4 = 0;
  uStack_10 = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,local_88);
  iStack_4._0_1_ = 1;
  uStack_6c = 0xf;
  bStack_89 = 0;
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,(char *)&bStack_89);
  uVar4 = std::char_traits<char>::length("Channel");
  FUN_004377d0(auStack_84,"Channel",uVar4);
  iStack_4._0_1_ = 2;
  bVar2 = Mercury__unknown_00e57390(param_1,auStack_84,&bStack_89);
  cStack_8c = !bVar2;
  iStack_4 = CONCAT31(iStack_4._1_3_,1);
  if (0xf < uStack_6c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_80[0]);
  }
  uStack_6c = 0xf;
  bStack_8b = 0;
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,(char *)&bStack_8b);
  if (cStack_8c != '\0') {
    FUN_00486000("aEvent->getProperty( \"Channel\", channel )",".\\Src\\Communicator.cpp",0x203);
  }
  uStack_6c = 0xf;
  cStack_8c = '\0';
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,&cStack_8c);
  uVar4 = std::char_traits<char>::length("Speaker");
  FUN_004377d0(auStack_84,"Speaker",uVar4);
  iStack_4._0_1_ = 3;
  bVar2 = Mercury__unknown_00cfc690(param_1,auStack_84,auStack_44);
  bStack_8b = !bVar2;
  iStack_4 = CONCAT31(iStack_4._1_3_,1);
  if (0xf < uStack_6c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_80[0]);
  }
  uStack_6c = 0xf;
  cStack_8c = '\0';
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,&cStack_8c);
  if (bStack_8b != 0) {
    FUN_00486000("aEvent->getProperty( \"Speaker\", speaker )",".\\Src\\Communicator.cpp",0x204);
  }
  uStack_6c = 0xf;
  cStack_8c = '\0';
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,&cStack_8c);
  uVar4 = std::char_traits<char>::length("SpeakerFlags");
  FUN_004377d0(auStack_84,"SpeakerFlags",uVar4);
  iStack_4._0_1_ = 4;
  bVar2 = Mercury__unknown_00e57390(param_1,auStack_84,&bStack_8b);
  cStack_8a = !bVar2;
  iStack_4 = CONCAT31(iStack_4._1_3_,1);
  if (0xf < uStack_6c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_80[0]);
  }
  uStack_6c = 0xf;
  cStack_8c = '\0';
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,&cStack_8c);
  if (cStack_8a != '\0') {
    FUN_00486000("aEvent->getProperty( \"SpeakerFlags\", speakerFlags )",".\\Src\\Communicator.cpp",
                 0x205);
  }
  uStack_6c = 0xf;
  cStack_8a = '\0';
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,&cStack_8a);
  uVar4 = std::char_traits<char>::length("Text");
  FUN_004377d0(auStack_84,"Text",uVar4);
  iStack_4._0_1_ = 5;
  bVar2 = Mercury__unknown_00cfc690(param_1,auStack_84,auStack_28);
  cStack_8c = !bVar2;
  iStack_4 = CONCAT31(iStack_4._1_3_,1);
  if (0xf < uStack_6c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_80[0]);
  }
  uStack_6c = 0xf;
  cStack_8a = '\0';
  uStack_70 = 0;
  std::char_traits<char>::assign((char *)auStack_80,&cStack_8a);
  if (cStack_8c != '\0') {
    FUN_00486000("aEvent->getProperty( \"Text\", text )",".\\Src\\Communicator.cpp",0x206);
  }
  uStack_48 = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  uStack_4c = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_5c,local_88);
  uVar4 = std::char_traits<wchar_t>::length(L"bleepSpecialWords");
  FUN_00438520(auStack_60,L"bleepSpecialWords",uVar4);
  iStack_4._0_1_ = 6;
  uStack_6c = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  uStack_70 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_80,local_88);
  uVar4 = std::char_traits<wchar_t>::length(L"ui");
  FUN_00438520(auStack_84,L"ui",uVar4);
  iStack_4._0_1_ = 7;
  puVar8 = auStack_60;
  puVar7 = auStack_84;
  ppuVar6 = &puStack_68;
  this = (void *)FUN_005757f0();
  FUN_00cfb560(this,(int *)ppuVar6,puVar7,puVar8);
  iStack_4._0_1_ = 9;
  if (7 < uStack_6c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_80[0]);
  }
  uStack_6c = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  uStack_70 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_80,local_88);
  iStack_4._0_1_ = 10;
  if (7 < uStack_48) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_5c[0]);
  }
  uStack_48 = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  uStack_4c = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_5c,local_88);
  puVar1 = puStack_68;
  iVar5 = FUN_005757f0();
  cVar3 = FUN_00574430(iVar5);
  if (cVar3 == '\0') {
    if (*(char *)((int)puVar1 + 0x61) == '\0') {
      cVar3 = *(char *)((int)puVar1 + 0xb3);
    }
    else {
      cVar3 = *(char *)((int)puVar1 + 0xb2);
    }
  }
  else {
    cVar3 = *(char *)((int)puVar1 + 0xb1);
  }
  if ((cVar3 == '\0') || (*(int *)((int)local_64 + 0x48) == 0)) {
    Mercury__unknown_00ceac90
              (local_64,(char *)(uint)bStack_89,auStack_44,(uint)bStack_8b,auStack_28);
  }
  else {
    uStack_48 = 7;
    local_88[0] = L'\0';
    local_88[1] = L'\0';
    uStack_4c = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_5c,local_88);
    iStack_4._0_1_ = 0xb;
    FUN_00ceb130(local_64,(int)auStack_28,auStack_60);
    Mercury__unknown_00ceac90
              (local_64,(char *)(uint)bStack_89,auStack_44,(uint)bStack_8b,auStack_60);
    iStack_4._0_1_ = 10;
    FUN_00433f40((int)auStack_60);
  }
  iStack_4._0_1_ = 1;
  if (((puStack_68 != (undefined4 *)0x0) &&
      (puStack_68[1] = puStack_68[1] + -1, (int)puStack_68[1] < 1)) &&
     (puStack_68 != (undefined4 *)0x0)) {
    (**(code **)*puStack_68)(1);
  }
  iStack_4 = (uint)iStack_4._1_3_ << 8;
  if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,local_88);
  iStack_4 = 0xffffffff;
  if (7 < local_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_40[0]);
  }
  local_2c = 7;
  local_88[0] = L'\0';
  local_88[1] = L'\0';
  local_30 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_40,local_88);
  ExceptionList = pvStack_c;
  return;
}


/* Mercury debug (Channel): aEvent->getProperty( "Channel", channel ) */

void Mercury_Channel_10(void *param_1)

{
  bool bVar1;
  char cVar2;
  uint uVar3;
  void *pvVar4;
  int iVar5;
  undefined4 extraout_ECX;
  int *piVar6;
  undefined4 *puVar7;
  undefined4 uVar8;
  undefined1 **ppuVar9;
  undefined4 *puVar10;
  uint *puVar11;
  undefined4 *puVar12;
  undefined1 *puVar13;
  undefined4 uStack_d0;
  undefined1 *local_cc;
  wchar_t awStack_c8 [2];
  char acStack_c4 [12];
  undefined4 uStack_b8;
  uint uStack_b4;
  uint uStack_b0;
  undefined4 uStack_ac;
  void *pvStack_a8;
  undefined **local_a4;
  uint uStack_a0;
  undefined1 auStack_9c [4];
  undefined1 auStack_98 [4];
  undefined4 uStack_94;
  undefined4 uStack_90;
  wchar_t awStack_8c [2];
  wchar_t awStack_88 [6];
  undefined4 uStack_7c;
  uint uStack_78;
  undefined4 uStack_74;
  wchar_t awStack_70 [2];
  wchar_t awStack_6c [6];
  undefined4 uStack_60;
  uint uStack_5c;
  undefined4 uStack_58;
  wchar_t awStack_54 [8];
  undefined4 uStack_44;
  uint uStack_40;
  undefined1 auStack_3c [4];
  wchar_t awStack_38 [2];
  wchar_t local_34 [6];
  undefined4 uStack_28;
  uint local_24;
  undefined4 local_20;
  undefined1 auStack_1c [12];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016f95f1;
  pvStack_c = ExceptionList;
  local_20 = 7;
  local_cc = (undefined1 *)0x0;
  local_24 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<wchar_t>::assign(local_34,(wchar_t *)&local_cc);
  uStack_4 = 0;
  uStack_58 = 7;
  local_cc = (undefined1 *)0x0;
  uStack_5c = 0;
  std::char_traits<wchar_t>::assign(awStack_6c,(wchar_t *)&local_cc);
  uStack_4._0_1_ = 1;
  uStack_74 = 7;
  local_cc = (undefined1 *)0x0;
  uStack_78 = 0;
  std::char_traits<wchar_t>::assign(awStack_88,(wchar_t *)&local_cc);
  uStack_4._0_1_ = 2;
  Detail__unknown_004405a0(&uStack_a0);
  uStack_4._0_1_ = 3;
  uStack_94 = FUN_00570310();
  uStack_90 = 0;
  uStack_4._0_1_ = 4;
  uStack_b0 = 0xf;
  uStack_d0 = uStack_d0 & 0xffffff;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 3));
  uVar3 = std::char_traits<char>::length("Channel");
  FUN_004377d0(awStack_c8,"Channel",uVar3);
  uStack_4._0_1_ = 5;
  bVar1 = Mercury__unknown_00e57390(param_1,awStack_c8,(undefined1 *)((int)&uStack_ac + 2));
  uStack_d0._0_3_ = CONCAT12(!bVar1,(wchar_t)uStack_d0);
  uStack_4 = CONCAT31(uStack_4._1_3_,4);
  if (0xf < uStack_b0) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_b0 = 0xf;
  uStack_d0 = (uint)(uint3)uStack_d0;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 3));
  if (uStack_d0._2_1_ != '\0') {
    FUN_00486000("aEvent->getProperty( \"Channel\", channel )",".\\Src\\Communicator.cpp",0x224);
  }
  uStack_b0 = 0xf;
  uStack_d0._0_3_ = (uint3)(ushort)(wchar_t)uStack_d0;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  uVar3 = std::char_traits<char>::length("Speaker");
  FUN_004377d0(awStack_c8,"Speaker",uVar3);
  uStack_4._0_1_ = 6;
  bVar1 = Mercury__unknown_00cfc690(param_1,awStack_c8,awStack_38);
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0);
  uStack_4 = CONCAT31(uStack_4._1_3_,4);
  if (0xf < uStack_b0) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_b0 = 0xf;
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0) & 0xff00ffff;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  if (uStack_d0._3_1_ != '\0') {
    FUN_00486000("aEvent->getProperty( \"Speaker\", speaker )",".\\Src\\Communicator.cpp",0x225);
  }
  uStack_b0 = 0xf;
  uStack_d0._0_3_ = (uint3)(ushort)(wchar_t)uStack_d0;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  uVar3 = std::char_traits<char>::length("SpeakerFlags");
  FUN_004377d0(awStack_c8,"SpeakerFlags",uVar3);
  uStack_4._0_1_ = 7;
  bVar1 = Mercury__unknown_00e57390(param_1,awStack_c8,(undefined1 *)((int)&uStack_ac + 3));
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0);
  uStack_4 = CONCAT31(uStack_4._1_3_,4);
  if (0xf < uStack_b0) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_b0 = 0xf;
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0) & 0xff00ffff;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  if (uStack_d0._3_1_ != '\0') {
    FUN_00486000("aEvent->getProperty( \"SpeakerFlags\", speakerFlags )",".\\Src\\Communicator.cpp",
                 0x226);
  }
  uStack_b0 = 0xf;
  uStack_d0._0_3_ = (uint3)(ushort)(wchar_t)uStack_d0;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  uVar3 = std::char_traits<char>::length("Text");
  FUN_004377d0(awStack_c8,"Text",uVar3);
  uStack_4._0_1_ = 8;
  bVar1 = Mercury__unknown_00cfc690(param_1,awStack_c8,awStack_70);
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0);
  uStack_4 = CONCAT31(uStack_4._1_3_,4);
  if (0xf < uStack_b0) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_b0 = 0xf;
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0) & 0xff00ffff;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  if (uStack_d0._3_1_ != '\0') {
    FUN_00486000("aEvent->getProperty( \"Text\", text )",".\\Src\\Communicator.cpp",0x227);
  }
  uStack_b0 = 0xf;
  uStack_d0._0_3_ = (uint3)(ushort)(wchar_t)uStack_d0;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  uVar3 = std::char_traits<char>::length("tokenList");
  FUN_004377d0(awStack_c8,"tokenList",uVar3);
  uStack_4._0_1_ = 9;
  bVar1 = Detail__unknown_00dfe8d0(param_1,awStack_c8,(int)&uStack_a0);
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0);
  uStack_4 = CONCAT31(uStack_4._1_3_,4);
  if (0xf < uStack_b0) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_b0 = 0xf;
  uStack_d0 = CONCAT13(!bVar1,(uint3)uStack_d0) & 0xff00ffff;
  uStack_b4 = 0;
  std::char_traits<char>::assign(acStack_c4,(char *)((int)&uStack_d0 + 2));
  if (uStack_d0._3_1_ != '\0') {
    FUN_00486000("aEvent->getChildList(\"tokenList\", stringTokenList)",".\\Src\\Communicator.cpp",
                 0x228);
  }
  puVar13 = auStack_98;
  puVar11 = &uStack_a0;
  FUN_00ea5a50(puVar11,puVar13);
  local_cc = &stack0xffffff14;
  uVar8 = extraout_ECX;
  FUN_00cfac30(&stack0xffffff14,(int)auStack_98);
  uStack_4 = CONCAT31((int3)((uint)uStack_4 >> 8),4);
  FUN_00ea0000(auStack_1c,uVar8,puVar11);
  puStack_8._0_1_ = 0xb;
  uStack_40 = 7;
  uStack_d0 = 0;
  uStack_44 = 0;
  std::char_traits<wchar_t>::assign(awStack_54,(wchar_t *)&uStack_d0);
  uVar3 = std::char_traits<wchar_t>::length(L"bleepSpecialWords");
  FUN_00438520(&uStack_58,L"bleepSpecialWords",uVar3);
  puStack_8._0_1_ = 0xc;
  uStack_b4 = 7;
  uStack_d0 = 0;
  uStack_b8 = 0;
  std::char_traits<wchar_t>::assign(awStack_c8,(wchar_t *)&uStack_d0);
  uVar3 = std::char_traits<wchar_t>::length(L"ui");
  FUN_00438520(&local_cc,L"ui",uVar3);
  puStack_8._0_1_ = 0xd;
  puVar10 = &uStack_58;
  ppuVar9 = &local_cc;
  piVar6 = &uStack_ac;
  pvVar4 = (void *)FUN_005757f0();
  FUN_00cfb560(pvVar4,piVar6,ppuVar9,puVar10);
  puStack_8._0_1_ = 0xf;
  if (7 < uStack_b4) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_b4 = 7;
  uStack_d0 = 0;
  uStack_b8 = 0;
  std::char_traits<wchar_t>::assign(awStack_c8,(wchar_t *)&uStack_d0);
  puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,0x10);
  if (7 < uStack_40) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_40 = 7;
  uStack_d0 = 0;
  uStack_44 = 0;
  std::char_traits<wchar_t>::assign(awStack_54,(wchar_t *)&uStack_d0);
  puVar10 = uStack_ac;
  iVar5 = FUN_005757f0();
  cVar2 = FUN_00574430(iVar5);
  if (cVar2 == '\0') {
    if (*(char *)((int)puVar10 + 0x61) == '\0') {
      cVar2 = *(char *)((int)puVar10 + 0xb3);
    }
    else {
      cVar2 = *(char *)((int)puVar10 + 0xb2);
    }
  }
  else {
    cVar2 = *(char *)((int)puVar10 + 0xb1);
  }
  if ((cVar2 != '\0') && (*(int *)((int)pvStack_a8 + 0x48) != 0)) {
    FUN_00ceb130(pvStack_a8,(int)&uStack_74,&uStack_90);
    FUN_00437900(&uStack_74,&uStack_90,0,0xffffffff);
    FUN_0042e960(&uStack_90);
  }
  puVar12 = &local_20;
  puVar10 = &uStack_90;
  puVar7 = &uStack_74;
  pvVar4 = FUN_00ea3e00();
  FUN_00ea38c0(pvVar4,(int)puVar7,puVar10,(int)puVar12,(int)puVar13);
  Mercury__unknown_00ceac90
            (pvStack_a8,(char *)(uStack_b0 >> 0x10 & 0xff),auStack_3c,uStack_b0 >> 0x18,&uStack_90);
  puStack_8._0_1_ = 0xb;
  if (((uStack_ac != (undefined4 *)0x0) && (uStack_ac[1] = uStack_ac[1] + -1, (int)uStack_ac[1] < 1)
      ) && (uStack_ac != (undefined4 *)0x0)) {
    (**(code **)*uStack_ac)();
  }
  puStack_8._0_1_ = 4;
  FUN_00e9f3d0((int)&local_20);
  puStack_8._0_1_ = 3;
  FUN_01605f50((int)auStack_9c);
  local_a4 = CME::
             BasicPropertyList<struct_TypeList::TypeList<unsigned_char,struct_TypeList::TypeList<char,struct_TypeList::TypeList<unsigned_short,struct_TypeList::TypeList<short,struct_TypeList::TypeList<unsigned_long,struct_TypeList::TypeList<long,struct_TypeList::TypeList<unsigned___int64,struct_TypeList::TypeList<__int64,struct_TypeList::TypeList<float,struct_TypeList::TypeList<class_std::basic_string<wchar_t,struct_std::char_traits<wchar_t>,class_tbb::scalable_allocator<wchar_t>_>,struct_TypeList::TypeList<class_Vector2,struct_TypeList::TypeList<class_Vector3,struct_TypeList::TypeList<class_Vector4,struct_TypeList::NullType>_>_>_>_>_>_>_>_>_>_>_>_>_>
             ::vftable;
  puStack_8._0_1_ = 2;
  Detail__unknown_0043d770(&uStack_a0);
  puStack_8._0_1_ = 1;
  if (7 < uStack_78) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_78 = 7;
  uStack_d0 = 0;
  uStack_7c = 0;
  std::char_traits<wchar_t>::assign(awStack_8c,(wchar_t *)&uStack_d0);
  puStack_8 = (undefined1 *)((uint)puStack_8._1_3_ << 8);
  if (7 < uStack_5c) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_5c = 7;
  uStack_d0 = 0;
  uStack_60 = 0;
  std::char_traits<wchar_t>::assign(awStack_70,(wchar_t *)&uStack_d0);
  puStack_8 = (undefined1 *)0xffffffff;
  if (7 < local_24) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  local_24 = 7;
  uStack_d0 = 0;
  uStack_28 = 0;
  std::char_traits<wchar_t>::assign(awStack_38,(wchar_t *)&uStack_d0);
  ExceptionList = pvStack_10;
  return;
}


/* Mercury debug (Channel): aEvent->getProperty( "ChannelName", channelName ) */

void __thiscall Mercury_Channel_11(void *this,void *param_1)

{
  bool bVar1;
  uint uVar2;
  void *pvVar3;
  void *this_00;
  undefined4 in_stack_ffffff4c;
  undefined4 in_stack_ffffff50;
  char cStack_83;
  char cStack_82;
  byte bStack_81;
  wchar_t local_80 [4];
  undefined1 *puStack_78;
  exception aeStack_74 [12];
  undefined1 auStack_68 [4];
  wchar_t local_64 [8];
  undefined4 local_54;
  uint local_50;
  undefined1 auStack_4c [4];
  char acStack_48 [16];
  undefined4 uStack_38;
  uint uStack_34;
  undefined1 auStack_2c [32];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016f9636;
  pvStack_c = ExceptionList;
  local_50 = 7;
  local_80[0] = L'\0';
  local_80[1] = L'\0';
  local_54 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<wchar_t>::assign(local_64,local_80);
  uStack_4 = 0;
  uStack_34 = 0xf;
  cStack_83 = '\0';
  uStack_38 = 0;
  std::char_traits<char>::assign(acStack_48,&cStack_83);
  uVar2 = std::char_traits<char>::length("ChannelName");
  FUN_004377d0(auStack_4c,"ChannelName",uVar2);
  uStack_4._0_1_ = 1;
  bVar1 = Mercury__unknown_00cfc690(param_1,auStack_4c,auStack_68);
  cStack_82 = !bVar1;
  uStack_4 = (uint)uStack_4._1_3_ << 8;
  if (0xf < uStack_34) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_34 = 0xf;
  cStack_83 = '\0';
  uStack_38 = 0;
  std::char_traits<char>::assign(acStack_48,&cStack_83);
  if (cStack_82 != '\0') {
    FUN_00486000("aEvent->getProperty( \"ChannelName\", channelName )",".\\Src\\Communicator.cpp",
                 0x316);
  }
  uStack_34 = 0xf;
  cStack_82 = '\0';
  uStack_38 = 0;
  std::char_traits<char>::assign(acStack_48,&cStack_82);
  uVar2 = std::char_traits<char>::length("ChannelID");
  FUN_004377d0(auStack_4c,"ChannelID",uVar2);
  uStack_4._0_1_ = 2;
  bVar1 = Mercury__unknown_00e57390(param_1,auStack_4c,&bStack_81);
  cStack_83 = !bVar1;
  uStack_4 = (uint)uStack_4._1_3_ << 8;
  if (0xf < uStack_34) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  uStack_34 = 0xf;
  cStack_82 = '\0';
  uStack_38 = 0;
  std::char_traits<char>::assign(acStack_48,&cStack_82);
  if (cStack_83 != '\0') {
    FUN_00486000("aEvent->getProperty( \"ChannelID\", channelID )",".\\Src\\Communicator.cpp",0x317)
    ;
  }
  puStack_78 = &stack0xffffff4c;
  local_80[2] = L'\0';
  local_80[3] = L'\0';
  std::char_traits<wchar_t>::assign((wchar_t *)&stack0xffffff50,local_80 + 2);
  FUN_00437900(&stack0xffffff4c,auStack_68,0,0xffffffff);
  uStack_4 = uStack_4 & 0xffffff00;
  pvVar3 = FUN_00ea18f0(auStack_2c,in_stack_ffffff4c,in_stack_ffffff50);
  uStack_4._0_1_ = 4;
  FUN_00ea13c0(auStack_4c,pvVar3);
  uStack_4._0_1_ = 5;
  FUN_005736f0(this,(undefined4 *)aeStack_74,auStack_4c);
  uStack_4._0_1_ = 4;
  FUN_00e9efc0((int)auStack_4c);
  uStack_4._0_1_ = 0;
  FUN_00e9efc0((int)auStack_2c);
  pvVar3 = (void *)scalable_malloc();
  if (pvVar3 == (void *)0x0) {
    FUN_004099f0(aeStack_74);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(aeStack_74,(ThrowInfo *)&DAT_01d65cc8);
  }
  uStack_4._0_1_ = 6;
  puStack_78 = pvVar3;
  pvVar3 = FUN_00cfa340(pvVar3,auStack_68,(uint)bStack_81);
  uStack_4 = (uint)uStack_4._1_3_ << 8;
  this_00 = (void *)thunk_FUN_0054c900();
  FUN_00cf9ba0(this_00,0,pvVar3,1);
  uStack_4 = 0xffffffff;
  if (7 < local_50) {
                    /* WARNING: Subroutine does not return */
    scalable_free();
  }
  local_50 = 7;
  local_80[2] = L'\0';
  local_80[3] = L'\0';
  local_54 = 0;
  std::char_traits<wchar_t>::assign(local_64,local_80 + 2);
  ExceptionList = pvStack_c;
  return;
}


/* Mercury debug (Channel): aEvent->getProperty( "ChannelName", channelName ) */

void __thiscall Mercury_Channel_12(void *this,void *param_1)

{
  bool bVar1;
  uint uVar2;
  void *pvVar3;
  void *this_00;
  char cStack_5d;
  void *local_5c;
  void *pvStack_58;
  int *piStack_54;
  exception aeStack_50 [4];
  int *piStack_4c;
  undefined1 auStack_44 [4];
  undefined4 local_40 [4];
  undefined4 local_30;
  uint local_2c;
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_016f970b;
  pvStack_c = ExceptionList;
  local_2c = 7;
  local_5c = (void *)0x0;
  local_30 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<wchar_t>::assign((wchar_t *)local_40,(wchar_t *)&local_5c);
  iStack_4 = 0;
  uStack_10 = 0xf;
  cStack_5d = '\0';
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,&cStack_5d);
  uVar2 = std::char_traits<char>::length("ChannelName");
  FUN_004377d0(auStack_28,"ChannelName",uVar2);
  iStack_4._0_1_ = 1;
  bVar1 = Mercury__unknown_00cfc690(param_1,auStack_28,auStack_44);
  cStack_5d = !bVar1;
  iStack_4 = (uint)iStack_4._1_3_ << 8;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_1 = (void *)((uint)param_1 & 0xffffff00);
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  if (cStack_5d != '\0') {
    FUN_00486000("aEvent->getProperty( \"ChannelName\", channelName )",".\\Src\\Communicator.cpp",
                 0x34c);
  }
  FUN_0057d910(this,&pvStack_58,auStack_44);
  piStack_4c = *(int **)((int)this + 4);
  if ((pvStack_58 == (void *)0x0) || (pvStack_58 != this)) {
    _invalid_parameter_noinfo();
  }
  if (piStack_54 == piStack_4c) {
    uStack_10 = 7;
    param_1 = (void *)0x0;
    uStack_14 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&param_1);
    uVar2 = std::char_traits<wchar_t>::length(L"cannot leave channel you are not in.");
    FUN_00438520(auStack_28,L"cannot leave channel you are not in.",uVar2);
    iStack_4._0_1_ = 2;
    Mercury__unknown_00ceae50(this,auStack_28);
    iStack_4 = (uint)iStack_4._1_3_ << 8;
    if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_24[0]);
    }
    uStack_10 = 7;
    param_1 = (void *)0x0;
    uStack_14 = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&param_1);
    iStack_4 = 0xffffffff;
    if (7 < local_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_40[0]);
    }
  }
  else {
    if (pvStack_58 == (void *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (piStack_54 == *(int **)((int)pvStack_58 + 4)) {
      _invalid_parameter_noinfo();
    }
    param_1 = (void *)piStack_54[10];
    FUN_00576070(this,aeStack_50,pvStack_58,piStack_54);
    pvVar3 = (void *)scalable_malloc(0x20);
    if (pvVar3 == (void *)0x0) {
      FUN_004099f0(aeStack_50);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(aeStack_50,(ThrowInfo *)&DAT_01d65cc8);
    }
    iStack_4._0_1_ = 3;
    local_5c = pvVar3;
    pvVar3 = FUN_00cfa340(pvVar3,auStack_44,param_1);
    iStack_4 = (uint)iStack_4._1_3_ << 8;
    this_00 = (void *)thunk_FUN_0054c900();
    FUN_00cf9ca0(this_00,0,pvVar3,1);
    iStack_4 = 0xffffffff;
    if (7 < local_2c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_40[0]);
    }
  }
  iStack_4 = 0xffffffff;
  local_2c = 7;
  param_1 = (void *)0x0;
  local_30 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)local_40,(wchar_t *)&param_1);
  ExceptionList = pvStack_c;
  return;
}


/* Mercury debug (Nub): ServerConnection::send: calling Nub::send
    */

int __fastcall Mercury_Nub_7(void *param_1)

{
  void *this;
  undefined4 *puVar1;
  int iVar2;
  int *piVar3;
  char *pcVar4;
  undefined1 *local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017055e2;
  local_c = ExceptionList;
  piVar3 = (int *)0x0;
  if (*(int *)((int)param_1 + 0x30c) == 0) {
    return -1;
  }
  ExceptionList = &local_c;
  if ((*(char *)((int)param_1 + 0x315) != '\0') &&
     (ExceptionList = &local_c, *(char *)((int)param_1 + 0x314) == '\0')) {
    ExceptionList = &local_c;
    this = (void *)FUN_00418e30(0x74);
    local_4 = 0;
    if (this != (void *)0x0) {
      piVar3 = Mercury__unknown_0157aa40(this,0,0);
    }
    local_4 = 1;
    ServerConnection__unknown_0157ad80(piVar3,DAT_01ef24e4);
    FUN_00de3260(piVar3,(int)param_1 + 4);
    local_14 = DAT_01ef2458;
    local_10 = 2;
    ServerConnection__unknown_00a35210
              ((int *)&local_14,"ServerConnection::send: calling Nub::send\n");
    pcVar4 = (char *)0x0;
    local_4._0_1_ = 2;
    local_14 = &stack0xffffffd8;
    puVar1 = (undefined4 *)Mercury_Channel_5((int)param_1);
    local_4 = CONCAT31(local_4._1_3_,1);
    Mercury_Nub_2((void *)((int)param_1 + 400),puVar1,piVar3,pcVar4);
  }
  local_4 = 0xffffffff;
  if (*(char *)((int)param_1 + 0x36e) != '\0') {
    *(char *)((int)param_1 + 0x36c) = *(char *)((int)param_1 + 0x36c) + '\x01';
    *(undefined1 *)((int)param_1 + 0x36e) = 0;
  }
  if (*(undefined8 **)((int)param_1 + 0x174) != (undefined8 *)0x0) {
    *(undefined8 *)((int)param_1 + 0x180) = **(undefined8 **)((int)param_1 + 0x174);
  }
  *(int *)((int)param_1 + 0x68) = *(int *)((int)param_1 + 0x68) + 1;
  *(int *)((int)param_1 + 0xa8) = *(int *)((int)param_1 + 0xa8) + 1;
  piVar3 = (int *)Mercury_Channel_2((int)param_1);
  iVar2 = (**(code **)(*piVar3 + 8))();
  *(int *)((int)param_1 + 0x88) = *(int *)((int)param_1 + 0x88) + iVar2 * 8 + 0xe0;
  pcVar4 = (char *)Mercury_Channel_4((int)param_1);
  iVar2 = Mercury__unknown_01576f90(pcVar4);
  if (iVar2 < 0) {
    local_14 = DAT_01ef2458;
    local_10 = 4;
    Mercury__unknown_00de15e0(iVar2);
    ServerConnection__unknown_00a35210
              ((int *)&local_14,"ServerConnection::send: Disconnecting since send failed for %s.\n")
    ;
    Mercury__unknown_00dd8630(param_1,'\x01');
  }
  ExceptionList = local_c;
  return iVar2;
}


/* Mercury debug (Nub): BaseAppLoginHandler::BaseAppLoginHandler: calling Nub::send
    */

undefined4 * __thiscall Mercury_Nub_8(void *this,int *param_1)

{
  int *piVar1;
  int iVar2;
  uint *puVar3;
  int *piVar4;
  undefined4 *puVar5;
  undefined4 unaff_retaddr;
  void *local_1c;
  undefined4 local_18 [2];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017063de;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  *(undefined ***)this = Mercury::ReplyMessageHandler::vftable;
  local_4 = 0;
  *(undefined ***)this = BaseAppLoginHandler::vftable;
  iVar2 = *param_1;
  piVar1 = (int *)((int)this + 4);
  *piVar1 = iVar2;
  local_1c = this;
  if (iVar2 != 0) {
    FUN_00457e40(iVar2 + 4);
  }
  local_4._0_1_ = 2;
  puVar3 = FUN_00de4b90((void *)*piVar1,(uint *)&param_1);
  *(uint *)((int)this + 8) = *puVar3 + 400;
  local_4._0_1_ = 2;
  ServerConnection__unknown_00c71580((uint *)&param_1);
  piVar4 = (int *)scalable_malloc();
  if (piVar4 == (int *)0x0) {
    FUN_004099f0((exception *)local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4._0_1_ = 4;
  param_1 = piVar4;
  piVar4 = Mercury__unknown_0157aa40(piVar4,0,0);
  local_4 = CONCAT31(local_4._1_3_,5);
  FUN_0157adc0(piVar4,DAT_01ef24e0,this,0);
  iVar2 = *piVar1;
  param_1 = *(int **)(iVar2 + 0x5c);
  puVar5 = (undefined4 *)(**(code **)(*piVar4 + 0x10))();
  *puVar5 = unaff_retaddr;
  FUN_00de3260(piVar4,iVar2 + 0x60);
  local_1c = DAT_01ef2458;
  local_18[0] = 2;
  ServerConnection__unknown_00a35210
            ((int *)&local_1c,"BaseAppLoginHandler::BaseAppLoginHandler: calling Nub::send\n");
  puStack_8 = (undefined1 *)CONCAT31((int3)((uint)puStack_8 >> 8),5);
  Mercury_Nub_2(*(void **)((int)this + 8),(undefined4 *)(*piVar1 + 0x4c),piVar4,(char *)0x0);
  ExceptionList = pvStack_10;
  return this;
}


/* Mercury error in Channel: Error_NoAnimChannelsLeft */

undefined4 Mercury_Channel_6(void)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  undefined4 *puVar5;
  undefined **ppuVar6;
  int *piVar7;
  int local_1fc;
  int local_1f8;
  undefined4 local_1f4;
  int iStack_1f0;
  int iStack_1ec;
  undefined4 uStack_1e8;
  int aiStack_1e4 [3];
  int aiStack_1d8 [3];
  wxDialog awStack_1cc [424];
  undefined **ppuStack_24;
  int iStack_20;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01724927;
  local_c = ExceptionList;
  DAT_01ef5ea4 = 0;
  DAT_01ef5ea8 = 0;
  ExceptionList = &local_c;
  iVar1 = FUN_00b1f880();
  iVar1 = *(int *)(iVar1 + 0x4c);
  if (iVar1 == 0) {
    FUN_00486000("mode != NULL",".\\Src\\InterpEditorTools.cpp",0x89a);
  }
  iVar1 = *(int *)(iVar1 + 0x40);
  if (iVar1 == 0) {
    FUN_00486000("InterpEd != NULL",".\\Src\\InterpEditorTools.cpp",0x89d);
  }
  iVar2 = FUN_007a0050(*(void **)(iVar1 + 0x1e8),*(int *)(iVar1 + 0x1f0));
  if (iVar2 == 0) {
    FUN_00486000("GrInst",".\\Src\\InterpEditorTools.cpp",0x8a0);
  }
  piVar7 = *(int **)(iVar2 + 0x40);
  if (piVar7 != (int *)0x0) {
    iVar3 = FUN_007aec00(*(int *)(iVar1 + 0x1f0));
    if (iVar3 == 0) {
      (**(code **)(*piVar7 + 0x244))(*(int *)(iVar1 + 0x1f0) + 0x58);
    }
    local_1fc = 0;
    local_1f8 = 0;
    local_1f4 = 0;
    local_4 = 1;
    (**(code **)(*piVar7 + 0x240))(&local_1fc);
    if (local_1f8 == 0) {
LAB_00f7ab13:
      local_4 = 0xffffffff;
      UEditorEngine__unknown_0055ce80(&local_1fc);
      ExceptionList = local_c;
      return 1;
    }
    iStack_1f0 = 0;
    iStack_1ec = 0;
    uStack_1e8 = 0;
    local_4._0_1_ = 3;
    iVar1 = 0;
    if (0 < local_1f8) {
      iVar3 = 0;
      do {
        iVar4 = FUN_007af1e0(*(void **)(iVar2 + 0x3c),*(int *)(iVar3 + local_1fc),
                             *(int *)(iVar3 + 4 + local_1fc));
        if (iVar4 < *(int *)(iVar3 + 8 + local_1fc)) {
          puVar5 = FUN_0049b190((void *)(iVar3 + local_1fc),aiStack_1e4);
          local_4._0_1_ = 4;
          if (puVar5[1] == 0) {
            ppuVar6 = &PTR_017f8e10;
          }
          else {
            ppuVar6 = (undefined **)*puVar5;
          }
          FUN_0041aab0(aiStack_1d8,(short *)ppuVar6);
          local_4._0_1_ = 5;
          CME_UIWidget__unknown_0041b810(&iStack_1f0,aiStack_1d8);
          local_4._0_1_ = 4;
          FUN_0041b420(aiStack_1d8);
          local_4._0_1_ = 3;
          FUN_0041b420(aiStack_1e4);
        }
        iVar1 = iVar1 + 1;
        iVar3 = iVar3 + 0xc;
      } while (iVar1 < local_1f8);
    }
    if (iStack_1ec == 0) {
      puVar5 = FUN_00489aa0(aiStack_1e4,"Error_NoAnimChannelsLeft",L"UnrealEd",(wchar_t *)0x0);
      local_4._0_1_ = 6;
      if (puVar5[1] == 0) {
        ppuVar6 = &PTR_017f8e10;
      }
      else {
        ppuVar6 = (undefined **)*puVar5;
      }
      FUN_00492470(0,(wchar_t *)ppuVar6);
      local_4 = CONCAT31(local_4._1_3_,3);
      FUN_0041b420(aiStack_1e4);
    }
    else {
      FUN_0108e790(awStack_1cc,0,1);
      local_4._0_1_ = 7;
      iVar2 = FUN_0108ec90(awStack_1cc,L"ChooseAnimSlot",L"SlotName",&iStack_1f0,0);
      iVar1 = DAT_01ef5ea4;
      if (iVar2 == 0x13ec) {
        if (iStack_20 == 0) {
          ppuStack_24 = &PTR_017f8e10;
        }
        piVar7 = FUN_0049e960(aiStack_1d8,(wchar_t *)ppuStack_24,1);
        DAT_01ef5ea4 = *piVar7;
        DAT_01ef5ea8 = piVar7[1];
        if ((DAT_01ef5ea4 != 0) || (iVar1 = 0, DAT_01ef5ea8 != 0)) {
          local_4._0_1_ = 3;
          FUN_00f3a700(awStack_1cc);
          local_4 = CONCAT31(local_4._1_3_,1);
          FUN_0041bb90(&iStack_1f0);
          goto LAB_00f7ab13;
        }
      }
      DAT_01ef5ea4 = iVar1;
      local_4 = CONCAT31(local_4._1_3_,3);
      FUN_00f3a700(awStack_1cc);
    }
    local_4 = CONCAT31(local_4._1_3_,1);
    FUN_0041bb90(&iStack_1f0);
    local_4 = 0xffffffff;
    UEditorEngine__unknown_0055ce80(&local_1fc);
  }
  ExceptionList = local_c;
  return 0;
}


/* Mercury error in Nub: BaseNub::queryMachinedInterface: Failed to send interface discovery message
   to b */

uint __thiscall Mercury_Nub_3(void *this,undefined4 *param_1)

{
  int iVar1;
  uint uVar2;
  u_long uVar3;
  SOCKET local_34;
  u_long local_30;
  undefined **local_2c;
  char local_28;
  undefined4 uStack_24;
  undefined1 local_20;
  undefined4 local_1c [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  int local_4;
  
  puStack_8 = &LAB_01793068;
  pvStack_c = ExceptionList;
  local_34 = 0xffffffff;
  local_4 = 0;
  ExceptionList = &pvStack_c;
  local_34 = socket(2,2,0);
  if ((local_34 == 0xffffffff) && (iVar1 = WSAGetLastError(), iVar1 == 0x276d)) {
    FUN_015848f0();
    local_34 = socket(2,2,0);
  }
  iVar1 = Mercury__unknown_015770b0("lo",&local_30);
  uVar3 = local_30;
  if (iVar1 != 0) {
    _00___FRawStaticIndexBuffer__vfunc_0();
    uVar3 = 0x100007f;
  }
  FUN_01584870(&local_34,*(u_short *)((int)this + 0x34),*(ushort *)((int)this + 0x36),uVar3);
  FUN_015857a0(local_1c);
  local_4._1_3_ = (uint3)((uint)local_4 >> 8);
  local_2c = Mercury::BaseNub::QueryInterfaceHandler::vftable;
  local_28 = '\0';
  local_20 = 0;
  local_4._0_1_ = 3;
  iVar1 = Mercury_MachineGuard(&local_34,uVar3,&local_2c);
  if (iVar1 == 0) {
    if (local_28 != '\0') {
      *param_1 = uStack_24;
      local_2c = MachineGuardMessage::ReplyHandler::vftable;
      local_4 = (uint)local_4._1_3_ << 8;
      Mercury__unknown_015854c0(local_1c);
      local_4 = 0xffffffff;
      iVar1 = closesocket(local_34);
      ExceptionList = pvStack_c;
      return CONCAT31((int3)((uint)iVar1 >> 8),1);
    }
    local_2c = MachineGuardMessage::ReplyHandler::vftable;
    local_4 = (uint)local_4._1_3_ << 8;
    Mercury__unknown_015854c0(local_1c);
  }
  else {
    _00___FRawStaticIndexBuffer__vfunc_0();
    local_2c = MachineGuardMessage::ReplyHandler::vftable;
    local_4 = (uint)local_4._1_3_ << 8;
    Mercury__unknown_015854c0(local_1c);
  }
  local_4 = 0xffffffff;
  uVar2 = closesocket(local_34);
  ExceptionList = pvStack_c;
  return uVar2 & 0xffffff00;
}


/* Mercury debug (Nub): BaseNub::recreateListeningSocket: Querying BWMachined for interface
    */

uint __thiscall Mercury_Nub_9(void *this,u_long param_1,SOCKET *param_2)

{
  u_long *puVar1;
  u_long uVar2;
  SOCKET *this_00;
  char *pcVar3;
  uint uVar4;
  undefined4 uVar5;
  char *pcVar6;
  int iVar7;
  int *piVar8;
  SOCKET s;
  sockaddr local_24;
  void *local_14;
  char local_10 [16];
  
  uVar2 = param_1;
  local_24.sa_family = 0;
  local_24.sa_data[0] = '\0';
  local_24.sa_data[1] = '\0';
  pcVar6 = (char *)(param_1 + 4);
  pcVar3 = pcVar6;
  if (0xf < *(uint *)(param_1 + 0x18)) {
    pcVar3 = *(char **)pcVar6;
  }
  local_14 = this;
  uVar4 = FUN_004242c0(&DAT_01e919c4,0,DAT_01e919d8,pcVar3,*(uint *)(param_1 + 0x14));
  this_00 = param_2;
  if (uVar4 == 0) {
    _00___FRawStaticIndexBuffer__vfunc_0();
    uVar5 = Mercury_Nub_3(this,(undefined4 *)&local_24);
    if ((char)uVar5 == '\0') {
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
  }
  else {
    if (0xf < *(uint *)(uVar2 + 0x18)) {
      pcVar6 = *(char **)pcVar6;
    }
    iVar7 = Mercury_Endpoint(pcVar6,local_10);
    if (iVar7 == 0) {
      _00___FRawStaticIndexBuffer__vfunc_0();
      iVar7 = Mercury__unknown_015770b0(local_10,(u_long *)&local_24);
    }
    else {
      iVar7 = *(int *)(uVar2 + 0x14);
    }
    if (iVar7 != 0) {
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
  }
  iVar7 = FUN_01584870(this_00,*(u_short *)((int)this + 0x2c),*(ushort *)((int)this + 0x2e),
                       local_24._0_4_);
  if (iVar7 == 0) {
    local_24.sa_family = 0;
    local_24.sa_data[0] = '\0';
    local_24.sa_data[1] = '\0';
    local_24.sa_data[2] = '\0';
    local_24.sa_data[3] = '\0';
    local_24.sa_data[4] = '\0';
    local_24.sa_data[5] = '\0';
    FUN_010360c0((void *)((int)this + 0x5c),(undefined4 *)&local_24);
    uVar4 = *(uint *)((int)this + 100);
    if (uVar4 < *(uint *)((int)this + 0x60)) {
      _invalid_parameter_noinfo();
    }
    local_24.sa_data._2_4_ = uVar4;
    if ((*(uint *)((int)this + 100) < uVar4 - 8) || (uVar4 - 8 < *(uint *)((int)this + 0x60))) {
      _invalid_parameter_noinfo();
    }
    puVar1 = (u_long *)(uVar4 - 8);
    if (*(u_long **)((int)this + 100) <= puVar1) {
      _invalid_parameter_noinfo();
    }
    param_1 = 0x10;
    iVar7 = getsockname(*this_00,&local_24,(int *)&param_1);
    if (iVar7 == 0) {
      if ((char *)(uVar4 - 4) != (char *)0x0) {
        *(undefined2 *)(uVar4 - 4) = local_24.sa_data._0_2_;
      }
      if (puVar1 != (u_long *)0x0) {
        *puVar1 = local_24.sa_data._2_4_;
      }
    }
    if (*puVar1 == 0) {
      iVar7 = FUN_01584810((undefined4 *)local_10);
      if ((iVar7 != 0) || (iVar7 = Mercury__unknown_015770b0(local_10,puVar1), iVar7 != 0)) {
        _00___FRawStaticIndexBuffer__vfunc_0();
        s = *this_00;
        goto LAB_01577cb0;
      }
      Mercury__unknown_015847f0(puVar1);
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
    Mercury__unknown_015847f0(puVar1);
    _00___FRawStaticIndexBuffer__vfunc_0();
    param_1 = 1;
    iVar7 = ioctlsocket(*this_00,-0x7ffb9982,&param_1);
    if (*(int *)((int)local_14 + 0x50) != 0) {
      iVar7 = FUN_01577770(local_14,(void *)((int)local_14 + 0x3c),
                           *(undefined4 *)((int)local_14 + 0x58),'\x01');
    }
    return CONCAT31((int3)((uint)iVar7 >> 8),1);
  }
  piVar8 = _errno();
  strerror(*piVar8);
  _00___FRawStaticIndexBuffer__vfunc_0();
  s = *this_00;
LAB_01577cb0:
  uVar4 = closesocket(s);
  if (uVar4 == 0) {
    *this_00 = 0xffffffff;
  }
  *this_00 = 0xffffffff;
  return uVar4 & 0xffffff00;
}


/* Mercury error in Bundle: Bundle::iterator::unpack( %s ): Error unpacking header length at %d
    */

int * __thiscall Mercury_Bundle(void *this,int *param_1)

{
  ushort uVar1;
  ushort uVar2;
  bool bVar3;
  bool bVar4;
  int *this_00;
  int iVar5;
  undefined4 *puVar6;
  int *piVar7;
  undefined4 uVar8;
  uint uVar9;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  this_00 = param_1;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017932b1;
  local_c = ExceptionList;
  bVar4 = false;
  uVar1 = *(ushort *)((int)this + 6);
  uVar2 = *(ushort *)((int)this + 6);
  ExceptionList = &local_c;
  iVar5 = Mercury__unknown_0158aa40((int)param_1);
  if ((int)(uint)*(ushort *)((int)this + 4) < (int)(iVar5 + (uint)uVar1)) {
    Mercury__unknown_0158aa40((int)this_00);
  }
  else {
    iVar5 = *(int *)this;
    *(undefined1 *)((int)this + 0x18) = *(undefined1 *)(iVar5 + 0x54 + (uint)uVar1);
    if (iVar5 != 0) {
      param_1 = (int *)(iVar5 + 4);
      LOCK();
      *param_1 = *param_1 + 1;
      UNLOCK();
    }
    local_4 = 0xffffffff;
    puVar6 = Mercury_InterfaceElement_expandLength(this_00,(uint)uVar2 + *(int *)this + 0x54);
    *(undefined4 **)((int)this + 0x20) = puVar6;
    if (puVar6 == (undefined4 *)0xffffffff) {
      _00___FRawStaticIndexBuffer__vfunc_0();
      goto LAB_015799e9;
    }
    iVar5 = Mercury__unknown_0158aa40((int)this_00);
    uVar9 = (uint)uVar2 + iVar5;
    if (*(short *)((int)this + 0x14) == *(short *)((int)this + 6)) {
      if ((uint)*(ushort *)((int)this + 4) < (uVar9 & 0xffff) + 6) goto LAB_015799e1;
      *(undefined4 *)((int)this + 0x1c) = *(undefined4 *)((uVar9 & 0xffff) + 0x54 + *(int *)this);
      *(undefined2 *)((int)this + 0x14) =
           *(undefined2 *)((uVar9 + 4 & 0xffff) + 0x54 + *(int *)this);
      uVar9 = uVar9 + 6;
      *(undefined1 *)((int)this + 0x19) = 1;
    }
    else {
      *(undefined1 *)((int)this + 0x19) = 0;
    }
    if ((int)(uint)*(ushort *)((int)this + 4) < (int)(*(int *)((int)this + 0x20) + (uVar9 & 0xffff))
       ) {
      piVar7 = Mercury__unknown_0158a850(*(void **)this,(int *)&param_1);
      bVar4 = true;
      if (*piVar7 != 0) goto LAB_015799a3;
      bVar3 = true;
    }
    else {
LAB_015799a3:
      bVar3 = false;
    }
    local_4 = 0xffffffff;
    if (bVar4) {
      Mercury__unknown_0157df90((int *)&param_1);
    }
    if (!bVar3) {
      *(short *)((int)this + 8) = (short)uVar9;
      iVar5 = *(int *)((int)this + 0x20);
      *(int *)((int)this + 0xc) = iVar5;
      uVar8 = FUN_01578e20(this_00,iVar5);
      if ((char)uVar8 == '\0') {
        *(int *)((int)this + 0xc) = iVar5 + 4;
      }
      ExceptionList = local_c;
      return (int *)((int)this + 0x18);
    }
  }
LAB_015799e1:
  _00___FRawStaticIndexBuffer__vfunc_0();
LAB_015799e9:
  *(undefined1 *)((int)this + 0x19) = 0x20;
  _00___FRawStaticIndexBuffer__vfunc_0();
  ExceptionList = local_c;
  return (int *)((int)this + 0x18);
}


/* Mercury debug (Bundle): Bundle::iterator::data: Run out of packets after %d of %d bytes put in
   temp
    */

int __fastcall Mercury_Bundle_3(int *param_1)

{
  bool bVar1;
  undefined4 *this;
  int *piVar2;
  int iVar3;
  uint uVar4;
  size_t _Size;
  undefined4 *local_30;
  uint local_2c;
  undefined4 *local_28;
  int local_24;
  int *local_20;
  int local_1c;
  int local_18;
  int local_14;
  int *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017932f9;
  local_c = ExceptionList;
  local_2c = 0;
  if ((int)(param_1[3] + (uint)*(ushort *)(param_1 + 2)) <= (int)(uint)*(ushort *)(param_1 + 1)) {
    return *(ushort *)(param_1 + 2) + 0x54 + *param_1;
  }
  ExceptionList = &local_c;
  piVar2 = Mercury__unknown_0158a850((void *)*param_1,(int *)&local_28);
  iVar3 = *piVar2;
  local_4 = 0xffffffff;
  if (local_28 != (undefined4 *)0x0) {
    local_20 = local_28 + 1;
    LOCK();
    local_24 = *local_20;
    *local_20 = *local_20 + -1;
    UNLOCK();
    if ((local_24 + -1 < 1) && (local_28 != (undefined4 *)0x0)) {
      (**(code **)*local_28)(1);
    }
  }
  if (iVar3 == 0) {
    ExceptionList = local_c;
    return 0;
  }
  if ((short)param_1[2] == (short)param_1[1]) {
    piVar2 = Mercury__unknown_0158a850((void *)*param_1,&local_1c);
    local_2c = 1;
    if (param_1[3] + 1 <= *(int *)(*piVar2 + 0x24)) {
      bVar1 = true;
      goto LAB_01579b2f;
    }
  }
  bVar1 = false;
LAB_01579b2f:
  local_4 = 0xffffffff;
  if ((local_2c & 1) != 0) {
    local_2c = local_2c & 0xfffffffe;
    Mercury__unknown_0157df90(&local_1c);
  }
  if (!bVar1) {
    iVar3 = FUN_00418e30(param_1[3]);
    param_1[4] = iVar3;
    Mercury__unknown_0157df60(&local_30,*param_1);
    local_4 = 3;
    uVar4 = (uint)*(ushort *)(param_1 + 2);
    iVar3 = 0;
    if (0 < param_1[3]) {
      do {
        this = local_30;
        if (local_30 == (undefined4 *)0x0) {
          _00___FRawStaticIndexBuffer__vfunc_0();
          local_4 = 0xffffffff;
          Mercury__unknown_0157df90((int *)&local_30);
          ExceptionList = local_c;
          return 0;
        }
        _Size = local_30[9] - uVar4 & 0xffff;
        if (param_1[3] - iVar3 < (int)_Size) {
          _Size = (size_t)(ushort)((short)param_1[3] - (short)iVar3);
        }
        memcpy((void *)(param_1[4] + iVar3),(void *)(uVar4 + 0x54 + (int)local_30),_Size);
        piVar2 = Mercury__unknown_0158a850(this,&local_14);
        local_4._0_1_ = 4;
        Mercury__unknown_01576d80(&local_30,piVar2);
        local_4 = CONCAT31(local_4._1_3_,3);
        Mercury__unknown_0157df90(&local_14);
        iVar3 = iVar3 + _Size;
        uVar4 = 1;
      } while (iVar3 < param_1[3]);
    }
    iVar3 = param_1[4];
    local_4 = 0xffffffff;
    if (local_30 != (undefined4 *)0x0) {
      local_10 = local_30 + 1;
      LOCK();
      local_20 = (int *)*local_10;
      *local_10 = *local_10 + -1;
      UNLOCK();
      if (local_20 == (int *)0x1 || (int)local_20 + -1 < 0) {
        (**(code **)*local_30)(1);
      }
    }
    ExceptionList = local_c;
    return iVar3;
  }
  piVar2 = Mercury__unknown_0158a850((void *)*param_1,&local_18);
  iVar3 = *piVar2;
  local_4 = 0xffffffff;
  Mercury__unknown_0157df90(&local_18);
  ExceptionList = local_c;
  return iVar3 + 0x55;
}


/* WARNING: Removing unreachable block (ram,0x0157a948) */
/* WARNING: Removing unreachable block (ram,0x0157a96d) */
/* Mercury debug (Bundle): Bundle::finalise: data not part of message found at end of bundle!
    */

int * __fastcall Mercury_Bundle_2(int *param_1)

{
  int *piVar1;
  int *piVar2;
  undefined4 *this;
  undefined4 *local_34;
  undefined4 *local_30;
  int local_2c;
  int local_28;
  int *local_24;
  int local_20;
  int *local_1c;
  int *local_18;
  int local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01793450;
  local_c = ExceptionList;
  piVar1 = ExceptionList;
  if ((char)param_1[3] != '\x01') {
    ExceptionList = &local_c;
    *(undefined1 *)(param_1 + 3) = 1;
    if ((param_1[0x19] == 0) && (*(uint *)(param_1[2] + 0x24) != (uint)*(ushort *)(param_1 + 0x1a)))
    {
      local_14 = 0;
      local_10 = 6;
      Mercury__unknown_00a351d0
                (&local_14,"Bundle::finalise: data not part of message found at end of bundle!\n");
    }
    FUN_0157a150(param_1);
    piVar1 = (int *)FUN_0157a0a0(param_1,'\0');
    if ((*(char *)((int)param_1 + 0xd) == '\0') && ((char)param_1[0xd] != '\0')) {
      piVar1 = (int *)FUN_013540f0(param_1 + 8);
    }
    local_30 = (undefined4 *)param_1[1];
    if (local_30 != (undefined4 *)0x0) {
      local_24 = local_30 + 1;
      LOCK();
      local_20 = *local_24;
      *local_24 = *local_24 + 1;
      UNLOCK();
      piVar1 = (int *)*local_24;
    }
    local_4 = 0;
    this = local_30;
    while (this != (undefined4 *)0x0) {
      if ((((param_1[9] != 0) && (param_1[10] - param_1[9] >> 3 != 0)) ||
          (*(char *)((int)param_1 + 0x6a) != '\0')) || (0 < param_1[0x1c])) {
        *(byte *)(this + 0x15) = *(byte *)(this + 0x15) | 0x10;
      }
      if ((*(byte *)(this + 0x15) & 0x40) == 0) {
        *(byte *)(this + 0x15) = *(byte *)(this + 0x15) | 0x40;
        this[10] = this[10] + 4;
      }
      piVar2 = Mercury__unknown_0158a850(this,(int *)&local_34);
      local_4._0_1_ = 1;
      piVar1 = piVar2;
      if (this != (undefined4 *)*piVar2) {
        local_1c = this + 1;
        LOCK();
        local_2c = *local_1c;
        *local_1c = *local_1c + -1;
        UNLOCK();
        piVar1 = (int *)(local_2c + -1);
        if ((int)piVar1 < 1) {
          piVar1 = (int *)(**(code **)*local_30)(1);
        }
        this = (undefined4 *)*piVar2;
        local_30 = this;
        if (this != (undefined4 *)0x0) {
          local_1c = this + 1;
          LOCK();
          piVar1 = (int *)*local_1c;
          *local_1c = *local_1c + 1;
          UNLOCK();
          local_18 = piVar1;
        }
      }
      local_4 = (uint)local_4._1_3_ << 8;
      if (local_34 != (undefined4 *)0x0) {
        local_1c = local_34 + 1;
        LOCK();
        local_28 = *local_1c;
        *local_1c = *local_1c + -1;
        UNLOCK();
        piVar1 = (int *)(local_28 + -1);
        this = local_30;
        if (((int)piVar1 < 1) && (local_34 != (undefined4 *)0x0)) {
          piVar1 = (int *)(**(code **)*local_34)(1);
          this = local_30;
        }
      }
    }
  }
  ExceptionList = local_c;
  return piVar1;
}


/* Mercury debug string: Mercury::Bundle::newMessage: tried to add a message with an unknown length
   format %d
    */

undefined1 * __thiscall Mercury_Bundle_newMessage(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  undefined1 *puVar5;
  int local_8 [2];
  
  iVar4 = Mercury__unknown_0158aa40(*(int *)((int)this + 0x58));
  if (iVar4 == -1) {
    local_8[0] = 0;
    local_8[1] = 6;
    Mercury__unknown_00a351d0
              (local_8,
               "Mercury::Bundle::newMessage: tried to add a message with an unknown length format %d\n"
              );
  }
  *(int *)((int)this + 0x6c) = *(int *)((int)this + 0x6c) + 1;
  if (*(char *)((int)this + 0x6a) != '\0') {
    *(int *)((int)this + 0x70) = *(int *)((int)this + 0x70) + 1;
  }
  iVar2 = *(int *)((int)this + 8);
  iVar1 = iVar4 + param_1;
  if (((0x5ad - *(int *)(iVar2 + 0x2c)) - *(int *)(iVar2 + 0x28)) - *(int *)(iVar2 + 0x24) < iVar1)
  {
    puVar5 = (undefined1 *)FUN_0157a5d0(this,iVar1);
  }
  else {
    iVar3 = *(int *)(iVar2 + 0x24);
    *(int *)(iVar2 + 0x24) = *(int *)(iVar2 + 0x24) + iVar1;
    puVar5 = (undefined1 *)(iVar3 + 0x54 + iVar2);
  }
  *(undefined1 **)((int)this + 100) = puVar5;
  *(undefined2 *)((int)this + 0x68) = *(undefined2 *)(*(int *)((int)this + 8) + 0x24);
  *puVar5 = **(undefined1 **)((int)this + 0x58);
  *(int *)((int)this + 0x60) = param_1;
  *(undefined4 *)((int)this + 0x5c) = 0;
  return puVar5 + iVar4;
}


/* Mercury debug string: Mercury::Nub::handleMessage: received the wrong kind of message!
    */

void __thiscall Mercury_Nub_handleMessage(void *this,undefined4 *param_1,char *param_2,int *param_3)

{
  int *piVar1;
  char *pcVar2;
  int *piVar3;
  void *pvVar4;
  undefined4 *puVar5;
  undefined4 uVar6;
  void *unaff_EDI;
  void *this_00;
  int *unaff_retaddr;
  void *local_20;
  int *piStack_1c;
  undefined1 auStack_18 [4];
  int *piStack_14;
  void *pvStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  piVar3 = param_3;
  pcVar2 = param_2;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0170e978;
  local_c = ExceptionList;
  local_20 = this;
  if (*param_2 != -1) {
    ExceptionList = &local_c;
    _00___FRawStaticIndexBuffer__vfunc_0();
    ExceptionList = local_c;
    return;
  }
  if (*(int *)(param_2 + 8) < 4) {
    ExceptionList = &local_c;
    _00___FRawStaticIndexBuffer__vfunc_0();
    ExceptionList = local_c;
    return;
  }
  ExceptionList = &local_c;
  puVar5 = (undefined4 *)(**(code **)(*param_3 + 4))(4);
  param_1 = (undefined4 *)*puVar5;
  piVar1 = (int *)(pcVar2 + 8);
  *piVar1 = *piVar1 + -4;
  this_00 = (void *)((int)this + 0x2c);
  ServerConnection__unknown_00e221a0(this_00,(int *)&local_20,(int *)&param_1);
  pvVar4 = local_20;
  piStack_14 = *(int **)((int)this + 0x30);
  if ((local_20 == (void *)0x0) || (local_20 != this_00)) {
    _invalid_parameter_noinfo();
  }
  if (piStack_1c == piStack_14) {
    _00___FRawStaticIndexBuffer__vfunc_0();
    (**(code **)(*piVar3 + 0xc))();
    ExceptionList = pvStack_10;
    return;
  }
  if (pvVar4 == (void *)0x0) {
    _invalid_parameter_noinfo();
  }
  if (piStack_1c == *(int **)((int)pvVar4 + 4)) {
    _invalid_parameter_noinfo();
  }
  puVar5 = (undefined4 *)piStack_1c[4];
  if (puVar5[8] != 0) {
    uVar6 = FUN_00de15b0(unaff_retaddr,puVar5 + 8);
    if ((char)uVar6 != '\0') {
      Mercury__unknown_015847f0(unaff_retaddr);
      _00___FRawStaticIndexBuffer__vfunc_0();
      ExceptionList = pvStack_10;
      return;
    }
  }
  if (puVar5[2] != -1) {
    Mercury__unknown_01578670(puVar5[2]);
    puVar5[2] = 0xffffffff;
  }
  FUN_00c6a1e0(this_00,auStack_18,local_20,piStack_1c);
  puStack_8 = (undefined1 *)0x0;
  param_1 = puVar5;
  (**(code **)(*(int *)puVar5[3] + 4))(unaff_retaddr,pcVar2,piVar3,puVar5[4],param_3);
  piStack_1c = (int *)0xffffffff;
  (**(code **)*puVar5)(1);
  ExceptionList = unaff_EDI;
  return;
}


/* Mercury error in Nub: Nub::processOrderedPacket( %s ): Discarding bundle due to corrupted header
   for m */

undefined4 __thiscall Mercury_Nub_processOrderedPacket(void *this,int param_1)

{
  int *piVar1;
  char cVar2;
  undefined2 uVar3;
  undefined4 uVar4;
  int *piVar5;
  int iVar6;
  uint uVar7;
  int local_94;
  undefined4 local_90;
  undefined **local_8c;
  undefined1 local_88;
  int local_84;
  int local_80;
  char local_7c [4];
  undefined *local_78;
  ulonglong local_74;
  ulonglong local_6c;
  ulonglong local_64;
  int local_5c [10];
  int local_34 [10];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
                    /* Nub::processOrderedPacket - Main Mercury message dispatch. Reads 1-byte
                       msg_id, indexes into InterfaceElement array (stride 0x24), dispatches to
                       handler vtable. System msgs 0x00-0x7F use table; entity methods >= 0x80 use
                       WORD_LENGTH. */
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01793681;
  pvStack_c = ExceptionList;
  if (*(char *)(param_1 + 0x14) == '\0') {
    local_94 = 0;
    ExceptionList = &pvStack_c;
  }
  else {
    ExceptionList = &pvStack_c;
    local_94 = Mercury__unknown_0157c740(this,(uint *)(param_1 + 8));
  }
  *(undefined1 *)((int)this + 0xc2) = 0;
  local_78 = &DAT_01e91b9c;
  iVar6 = *(int *)(*(int *)(param_1 + 0x10) + 4);
  local_90 = 0;
  FUN_0158a5f0(iVar6);
  Mercury__unknown_0158a1c0(iVar6);
  Mercury__unknown_015a40d0();
  local_74 = Mercury__unknown_012379f6();
  iVar6 = *(int *)(*(int *)(param_1 + 0x10) + 4);
  FUN_0158a720(iVar6);
  Mercury__unknown_0158a1c0(iVar6);
  Mercury__unknown_015a40d0();
  local_6c = Mercury__unknown_012379f6();
  rdtsc();
  Mercury__unknown_0158a1c0(*(int *)(*(int *)(param_1 + 0x10) + 4));
  Mercury__unknown_015a40d0();
  local_64 = Mercury__unknown_012379f6();
  local_7c[0] = '\x01';
  if (*(int **)((int)this + 0xc4) != (int *)0x0) {
    (**(code **)(**(int **)((int)this + 0xc4) + 4))();
  }
  FUN_0157a1b0(*(void **)(param_1 + 0x10),local_5c);
  local_4 = 0;
  FUN_0157a260(local_34);
  local_4 = CONCAT31(local_4._1_3_,1);
  uVar4 = Mercury__unknown_01578df0(local_5c,local_34);
  cVar2 = (char)uVar4;
  while( true ) {
    if ((cVar2 == '\0') || (*(char *)((int)this + 0xc2) != '\0')) goto LAB_0157cc88;
    uVar3 = Mercury__unknown_01578f50(local_5c);
    uVar7 = (uint)(byte)uVar3;
    if ((*(int *)((int)this + 0xc) == 0) ||
       ((uint)((*(int *)((int)this + 0x10) - *(int *)((int)this + 0xc)) / 0x24) <= uVar7)) {
      _invalid_parameter_noinfo();
    }
    piVar1 = (int *)(*(int *)((int)this + 0xc) + uVar7 * 0x24);
    if (*(int *)(*(int *)((int)this + 0xc) + 0xc + uVar7 * 0x24) == 0) break;
    piVar5 = Mercury_Bundle(local_5c,piVar1);
    piVar5[3] = (int)this;
    if ((*piVar5 & 0x2000) != 0) {
      if (local_94 == 0) {
        Mercury__unknown_015847f0((void *)(param_1 + 8));
      }
      else {
        Mercury__unknown_015767e0(local_94);
      }
      Mercury__unknown_01578f50(local_5c);
      _00___FRawStaticIndexBuffer__vfunc_0();
      LOCK();
      *(int *)((int)this + 0xf8) = *(int *)((int)this + 0xf8) + 1;
      UNLOCK();
LAB_0157cc7e:
      local_90 = 0xfffffffc;
LAB_0157cc88:
      uVar4 = Mercury__unknown_01578df0(local_5c,local_34);
      if (((char)uVar4 == '\0') || (*(char *)((int)this + 0xc2) != '\0')) {
        *(int *)((int)this + 0x100) = *(int *)((int)this + 0x100) + 1;
      }
      else {
        *(int *)((int)this + 0x118) = *(int *)((int)this + 0x118) + 1;
      }
      if (*(int **)((int)this + 0xc4) != (int *)0x0) {
        (**(code **)(**(int **)((int)this + 0xc4) + 8))();
      }
      local_4 = local_4 & 0xffffff00;
      FUN_01578f30((int)local_34);
      local_4 = 0xffffffff;
      FUN_01578f30((int)local_5c);
      ExceptionList = pvStack_c;
      return local_90;
    }
    iVar6 = Mercury_Bundle_3(local_5c);
    if (iVar6 == 0) {
      if (local_94 == 0) {
        Mercury__unknown_015847f0((void *)(param_1 + 8));
      }
      else {
        Mercury__unknown_015767e0(local_94);
      }
      Mercury__unknown_01578f50(local_5c);
      _00___FRawStaticIndexBuffer__vfunc_0();
      LOCK();
      *(int *)((int)this + 0xf8) = *(int *)((int)this + 0xf8) + 1;
      UNLOCK();
      goto LAB_0157cc7e;
    }
    local_88 = 0;
    local_80 = piVar5[2] + iVar6;
    local_8c = MemoryIStream::vftable;
    local_4 = CONCAT31((int3)(local_4 >> 8),3);
    *(int *)((int)this + 0x10c) = *(int *)((int)this + 0x10c) + 1;
    piVar1[8] = piVar1[8] + 1;
    piVar1[7] = piVar1[7] + piVar5[2];
    local_84 = iVar6;
    (**(code **)(*(int *)piVar1[3] + 4))((void *)(param_1 + 8),piVar5,&local_8c,local_7c);
    if (local_7c[0] != '\0') {
      local_7c[0] = '\0';
    }
    FUN_01579cd0(local_5c);
    if (local_80 != local_84) {
      if (local_94 == 0) {
        Mercury__unknown_015847f0((void *)(param_1 + 8));
      }
      else {
        Mercury__unknown_015767e0(local_94);
      }
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
    local_4._1_3_ = (undefined3)(local_4 >> 8);
    local_8c = MemoryIStream::vftable;
    local_4._0_1_ = 4;
    if (local_84 != local_80) {
      Mercury__unknown_00a353d0("MemoryIStream::~MemoryIStream: There are still %d bytes left\n");
    }
    local_4 = CONCAT31(local_4._1_3_,1);
    local_8c = BinaryIStream::vftable;
    uVar4 = Mercury__unknown_01578df0(local_5c,local_34);
    cVar2 = (char)uVar4;
  }
  if (local_94 == 0) {
    Mercury__unknown_015847f0((void *)(param_1 + 8));
  }
  else {
    Mercury__unknown_015767e0(local_94);
  }
  Mercury__unknown_01578f50(local_5c);
  _00___FRawStaticIndexBuffer__vfunc_0();
  local_90 = 0xfffffffb;
  goto LAB_0157cc88;
}


/* Mercury debug (Channel): Nub::addChannelToConnection: removing channel %s from connection %s to
   remap it
    */

void __thiscall Mercury_Channel_14(void *this,int *param_1,undefined4 *param_2)

{
  undefined4 *puVar1;
  undefined4 *puVar2;
  int iVar3;
  uint *puVar4;
  uint local_30;
  uint local_2c;
  undefined4 *local_28;
  int local_24;
  int local_20;
  char local_1c;
  undefined4 local_18 [2];
  undefined4 *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017937a8;
  local_c = ExceptionList;
  puVar1 = (undefined4 *)*param_1;
  ExceptionList = &local_c;
  if (puVar1 != (undefined4 *)0x0) {
    ExceptionList = &local_c;
    puVar1[1] = puVar1[1] + 1;
  }
  local_4 = 0xffffffff;
  puVar4 = FUN_0157bc70(local_18,*param_2,param_2[1],puVar1);
  puVar1 = (undefined4 *)puVar4[2];
  local_30 = *puVar4;
  local_2c = puVar4[1];
  if (puVar1 != (undefined4 *)0x0) {
    puVar1[1] = puVar1[1] + 1;
  }
  local_4._0_1_ = 3;
  local_4._1_3_ = 0;
  local_28 = puVar1;
  FUN_0157d350((void *)((int)this + 0x88),&local_24,&local_30);
  local_4 = CONCAT31(local_4._1_3_,1);
  if ((puVar1 != (undefined4 *)0x0) && (puVar1[1] = puVar1[1] + -1, (int)puVar1[1] < 1)) {
    (**(code **)*puVar1)();
  }
  local_4 = 0xffffffff;
  if (((local_10 != (undefined4 *)0x0) && (local_10[1] = local_10[1] + -1, (int)local_10[1] < 1)) &&
     (local_10 != (undefined4 *)0x0)) {
    (**(code **)*local_10)();
  }
  if (local_1c == '\0') {
    if (local_24 == 0) {
      _invalid_parameter_noinfo();
    }
    if (local_20 == *(int *)(local_24 + 4)) {
      _invalid_parameter_noinfo();
    }
    puVar1 = *(undefined4 **)(local_20 + 0x14);
    if (puVar1 != (undefined4 *)0x0) {
      puVar1[1] = puVar1[1] + 1;
    }
    local_4 = 6;
    puVar1[8] = puVar1[8] + -1;
    Mercury__unknown_015847f0(puVar1 + 3);
    Mercury__unknown_015847f0(param_2);
    _00___FRawStaticIndexBuffer__vfunc_0();
    if (local_20 == *(int *)(local_24 + 4)) {
      _invalid_parameter_noinfo();
    }
    puVar2 = *(undefined4 **)(local_20 + 0x14);
    if (puVar2 != (undefined4 *)*param_1) {
      if (((puVar2 != (undefined4 *)0x0) && (puVar2[1] = puVar2[1] + -1, (int)puVar2[1] < 1)) &&
         (puVar2 != (undefined4 *)0x0)) {
        (**(code **)*puVar2)();
      }
      iVar3 = *param_1;
      *(int *)(local_20 + 0x14) = iVar3;
      if (iVar3 != 0) {
        *(int *)(iVar3 + 4) = *(int *)(iVar3 + 4) + 1;
      }
    }
    local_4 = 0xffffffff;
    puVar1[1] = puVar1[1] + -1;
    if ((int)puVar1[1] < 1) {
      (**(code **)*puVar1)();
    }
  }
  *(int *)(*param_1 + 0x20) = *(int *)(*param_1 + 0x20) + 1;
  Mercury__unknown_015847f0((void *)(*param_1 + 0xc));
  Mercury__unknown_015847f0(param_2);
  _00___FRawStaticIndexBuffer__vfunc_0();
  ExceptionList = local_c;
  return;
}


/* Mercury debug (Nub): Nub::_processMessage: removing channel %s from connection %s; %d channels
   are us */

void __thiscall Mercury_Nub_10(void *this,uint *param_1)

{
  void *this_00;
  int *piVar1;
  void *local_8;
  int *local_4;
  
  this_00 = (void *)((int)this + 0x88);
  Mercury__unknown_0157c050(this_00,&local_8,param_1);
  piVar1 = *(int **)((int)this + 0x8c);
  if (this_00 != local_8) {
    _invalid_parameter_noinfo();
  }
  if (piVar1 != local_4) {
    if (local_8 == (void *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (local_4 == *(int **)((int)local_8 + 4)) {
      _invalid_parameter_noinfo();
    }
    *(int *)(local_4[5] + 0x20) = *(int *)(local_4[5] + 0x20) + -1;
    if (local_4 == *(int **)((int)local_8 + 4)) {
      _invalid_parameter_noinfo();
    }
    if (local_4 == *(int **)((int)local_8 + 4)) {
      _invalid_parameter_noinfo();
    }
    Mercury__unknown_015847f0((void *)(local_4[5] + 0xc));
    Mercury__unknown_015847f0(param_1);
    _00___FRawStaticIndexBuffer__vfunc_0();
    FUN_0157d430(this_00,&local_8,local_8,local_4);
  }
  return;
}


/* Mercury debug (Channel): Nub::_processMessage: registering ChannelInternal from address %s
    */

void __thiscall Mercury_Channel_13(void *this,int param_1)

{
  int *piVar1;
  void *pvVar2;
  uint *puVar3;
  undefined4 *puVar4;
  void *local_8;
  int *local_4;
  
  if (*(int *)(param_1 + 8) == 0) {
    puVar3 = (uint *)(param_1 + 0xc);
    Mercury_Nub_10(this,puVar3);
    pvVar2 = (void *)((int)this + 0xd4);
    Mercury__unknown_0157c050(pvVar2,&local_8,puVar3);
    piVar1 = *(int **)((int)this + 0xd8);
    if ((local_8 == (void *)0x0) || (local_8 != pvVar2)) {
      _invalid_parameter_noinfo();
    }
    if (local_4 != piVar1) {
      Mercury__unknown_015847f0(puVar3);
      _00___FRawStaticIndexBuffer__vfunc_0();
      if (local_8 == (void *)0x0) {
        _invalid_parameter_noinfo();
      }
      if (local_4 == *(int **)((int)local_8 + 4)) {
        _invalid_parameter_noinfo();
      }
      if ((undefined4 *)local_4[5] != (undefined4 *)0x0) {
        (*(code *)**(undefined4 **)local_4[5])(1);
      }
      FUN_0155bb10(pvVar2,&local_8,local_8,local_4);
    }
    return;
  }
  pvVar2 = (void *)Mercury__unknown_0158b960(*(int *)(param_1 + 8));
  Mercury__unknown_015847f0(pvVar2);
  _00___FRawStaticIndexBuffer__vfunc_0();
  pvVar2 = (void *)((int)this + 0xd4);
  puVar3 = (uint *)Mercury__unknown_0158b960(*(int *)(param_1 + 8));
  Mercury__unknown_0157c050(pvVar2,&local_8,puVar3);
  piVar1 = *(int **)((int)this + 0xd8);
  if ((local_8 == (void *)0x0) || (local_8 != pvVar2)) {
    _invalid_parameter_noinfo();
  }
  if (local_4 != piVar1) {
    if (local_8 == (void *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (local_4 == *(int **)((int)local_8 + 4)) {
      _invalid_parameter_noinfo();
    }
    if ((undefined4 *)local_4[5] != (undefined4 *)0x0) {
      (*(code *)**(undefined4 **)local_4[5])(1);
    }
    if (local_4 == *(int **)((int)local_8 + 4)) {
      _invalid_parameter_noinfo();
    }
    local_4[5] = *(int *)(param_1 + 8);
    return;
  }
  puVar3 = (uint *)Mercury__unknown_0158b960(*(int *)(param_1 + 8));
  puVar4 = FUN_0157ce60(pvVar2,puVar3);
  *puVar4 = *(undefined4 *)(param_1 + 8);
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* Mercury source: nub.cpp
   [String discovery] References: "Nub::registerChannel" */

uint __thiscall Mercury_Nub(void *this,undefined4 *param_1)

{
  uint *puVar1;
  int *piVar2;
  undefined4 *puVar3;
  int iVar4;
  undefined4 *puVar5;
  uint uVar6;
  void *this_00;
  int local_2c [2];
  exception local_24 [12];
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puVar3 = param_1;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017938ae;
  local_c = ExceptionList;
  puVar1 = param_1 + 8;
  ExceptionList = &local_c;
  iVar4 = Mercury__unknown_0157c740(this,puVar1);
  if (iVar4 != 0) {
    puVar5 = (undefined4 *)Mercury__unknown_0157c740(this,puVar1);
    if (puVar5 != puVar3) {
      local_2c[0] = 0;
      local_2c[1] = 6;
      uVar6 = FUN_00a351f0(local_2c,
                           "MF_ASSERT_DEV FAILED: findChannel(channel.addr()) == NULL || findChannel(channel.addr()) == &channel\n.\\nub.cpp(%d)%s%s\n"
                          );
      ExceptionList = local_c;
      return uVar6 & 0xffffff00;
    }
  }
  Mercury__unknown_015847f0(puVar1);
  _00___FRawStaticIndexBuffer__vfunc_0();
  puVar5 = FUN_0157ce60((void *)((int)this + 200),puVar1);
  *puVar5 = puVar3;
  puVar5 = (undefined4 *)scalable_malloc(0x180);
  if (puVar5 == (undefined4 *)0x0) {
    FUN_004099f0(local_24);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_24,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  param_1 = puVar5;
  puVar5 = Mercury__unknown_0158c7b0(puVar5,puVar3);
  local_4 = 0xffffffff;
  puVar3[3] = puVar5;
  this_00 = (void *)scalable_malloc(0x14);
  if (this_00 == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 1;
  param_1 = (undefined4 *)FUN_0158d8a0(this_00,puVar5);
  if (param_1 != (undefined4 *)0x0) {
    LOCK();
    param_1[1] = param_1[1] + 1;
    UNLOCK();
  }
  local_4 = 2;
  tbb::internal::concurrent_queue_base_v3::internal_push
            ((concurrent_queue_base_v3 *)((int)this + 0x150),&param_1);
  local_4 = 0xffffffff;
  iVar4 = 0;
  if (param_1 != (undefined4 *)0x0) {
    piVar2 = param_1 + 1;
    LOCK();
    iVar4 = *piVar2;
    *piVar2 = *piVar2 + -1;
    UNLOCK();
    iVar4 = iVar4 + -1;
    if ((iVar4 < 1) && (param_1 != (undefined4 *)0x0)) {
      iVar4 = (**(code **)*param_1)(1);
    }
  }
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)iVar4 >> 8),1);
}


/* Mercury debug (Channel): Nub::deregisterChannel: Channel not found %s!
    */

uint __thiscall Mercury_Channel_15(void *this,undefined4 *param_1)

{
  uint *puVar1;
  int *piVar2;
  undefined4 *puVar3;
  undefined4 *puVar4;
  uint uVar5;
  void *this_00;
  int iVar6;
  int local_20 [2];
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puVar3 = param_1;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017938d3;
  local_c = ExceptionList;
  puVar1 = param_1 + 8;
  ExceptionList = &local_c;
  puVar4 = (undefined4 *)Mercury__unknown_0157c740(this,puVar1);
  if (puVar3 != puVar4) {
    local_20[0] = 0;
    local_20[1] = 6;
    Mercury__unknown_015847f0(puVar1);
    uVar5 = Mercury__unknown_00a351d0(local_20,"Nub::deregisterChannel: Channel not found %s!\n");
    ExceptionList = local_c;
    return uVar5 & 0xffffff00;
  }
  Mercury__unknown_015847f0(puVar1);
  _00___FRawStaticIndexBuffer__vfunc_0();
  FUN_0157cef0((void *)((int)this + 200),puVar1);
  puVar3[3] = 0;
  this_00 = (void *)scalable_malloc(0x14);
  if (this_00 == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  param_1 = (undefined4 *)FUN_0158d910(this_00,puVar1);
  if (param_1 != (undefined4 *)0x0) {
    LOCK();
    param_1[1] = param_1[1] + 1;
    UNLOCK();
  }
  local_4 = 1;
  tbb::internal::concurrent_queue_base_v3::internal_push
            ((concurrent_queue_base_v3 *)((int)this + 0x150),&param_1);
  local_4 = 0xffffffff;
  iVar6 = 0;
  if (param_1 != (undefined4 *)0x0) {
    piVar2 = param_1 + 1;
    LOCK();
    iVar6 = *piVar2;
    *piVar2 = *piVar2 + -1;
    UNLOCK();
    iVar6 = iVar6 + -1;
    if ((iVar6 < 1) && (param_1 != (undefined4 *)0x0)) {
      iVar6 = (**(code **)*param_1)(1);
    }
  }
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)iVar6 >> 8),1);
}


/* WARNING: Removing unreachable block (ram,0x0157ed39) */
/* WARNING: Removing unreachable block (ram,0x0157ecff) */
/* Mercury source: nub.cpp */

undefined4 __thiscall
Mercury_Nub_2(void *this,undefined4 *param_1,undefined4 *param_2,char *param_3)

{
  int *piVar1;
  int iVar2;
  undefined8 uVar3;
  undefined4 uVar4;
  undefined4 *puVar5;
  undefined4 *puVar6;
  longlong *plVar7;
  void *this_00;
  undefined4 *puVar8;
  bool bVar9;
  undefined4 *local_3c;
  char *pcStack_38;
  int iStack_34;
  int *piStack_30;
  int local_2c [2];
  undefined **appuStack_24 [3];
  undefined **appuStack_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_01793928;
  pvStack_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &pvStack_c;
  uVar4 = FUN_01578e70((int)param_2);
  if ((char)uVar4 != '\0') {
    local_2c[0] = 0;
    local_2c[1] = 6;
    FUN_00a351f0(local_2c,"MF_ASSERT_DEV FAILED: !bundle->dataPastEnd()\n.\\nub.cpp(%d)%s%s\n");
  }
  uVar4 = FUN_01578e70((int)param_2);
  if ((char)uVar4 == '\0') {
    puVar8 = (undefined4 *)param_2[5];
    if ((undefined4 *)param_2[6] < puVar8) {
      _invalid_parameter_noinfo();
    }
    while( true ) {
      local_3c = (undefined4 *)param_2[6];
      if (local_3c < (undefined4 *)param_2[5]) {
        _invalid_parameter_noinfo();
      }
      if (puVar8 == local_3c) break;
      puVar5 = (undefined4 *)scalable_malloc();
      if (puVar5 == (undefined4 *)0x0) {
        param_3 = "scalable_malloc failed";
        std::exception::exception((exception *)appuStack_24,&param_3);
        appuStack_24[0] = std::bad_alloc::vftable;
        local_4 = local_4 & 0xffffff00;
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(appuStack_24,(ThrowInfo *)&DAT_01d65cc8);
      }
      *puVar5 = Mercury::TimerExpiryHandler::vftable;
      *puVar5 = Mercury::Nub::ReplyHandlerElement::vftable;
      puVar5[8] = 0;
      *(undefined2 *)(puVar5 + 9) = 0;
      *(undefined2 *)((int)puVar5 + 0x26) = 0;
      local_4 = local_4 & 0xffffff00;
      if (1000000 < *(int *)((int)this + 0x94)) {
        *(undefined4 *)((int)this + 0x94) = 1;
      }
      iVar2 = *(int *)((int)this + 0x94);
      *(int *)((int)this + 0x94) = iVar2 + 1;
      piVar1 = puVar5 + 1;
      *piVar1 = iVar2;
      puVar5[2] = 0xffffffff;
      piStack_30 = puVar5;
      if ((undefined4 *)param_2[6] <= puVar8) {
        _invalid_parameter_noinfo();
      }
      puVar5[3] = *puVar8;
      if ((undefined4 *)param_2[6] <= puVar8) {
        _invalid_parameter_noinfo();
      }
      puVar5[4] = puVar8[1];
      puVar5[5] = this;
      uVar3 = rdtsc();
      puVar5[6] = (int)uVar3;
      puVar5[7] = (int)((ulonglong)uVar3 >> 0x20);
      puVar5[8] = *param_1;
      puVar5[9] = param_1[1];
      puVar6 = ServerConnection__unknown_00575ff0((void *)((int)this + 0x9c),piVar1);
      *puVar6 = puVar5;
      if ((undefined4 *)param_2[6] <= puVar8) {
        _invalid_parameter_noinfo();
      }
      *(int *)puVar8[3] = *piVar1;
      if ((undefined4 *)param_2[6] <= puVar8) {
        _invalid_parameter_noinfo();
      }
      if (0 < (int)puVar8[2]) {
        if ((undefined4 *)param_2[6] <= puVar8) {
          _invalid_parameter_noinfo();
        }
        plVar7 = Mercury__unknown_01578ab0
                           ((void *)((int)this + 0x18),(longlong *)puVar8[2],puVar5,0,'\0');
        puVar5[2] = plVar7;
      }
      if ((undefined4 *)param_2[6] <= puVar8) {
        _invalid_parameter_noinfo();
      }
      puVar8 = puVar8 + 4;
    }
    this_00 = (void *)scalable_malloc();
    if (this_00 == (void *)0x0) {
      pcStack_38 = "scalable_malloc failed";
      std::exception::exception((exception *)appuStack_18,&pcStack_38);
      appuStack_18[0] = std::bad_alloc::vftable;
      local_4 = local_4 & 0xffffff00;
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(appuStack_18,(ThrowInfo *)&DAT_01d65cc8);
    }
    bVar9 = param_3 != (char *)0x0;
    param_3 = &stack0xffffffac;
    local_4._1_3_ = (uint3)(local_4 >> 8);
    local_4._0_1_ = 5;
    piStack_30 = this_00;
    local_3c = FUN_0158d750(this_00,param_1,param_2,bVar9);
    if (local_3c != (undefined4 *)0x0) {
      piVar1 = local_3c + 1;
      LOCK();
      iStack_34 = *piVar1;
      *piVar1 = *piVar1 + 1;
      UNLOCK();
    }
    local_4._0_1_ = 7;
    tbb::internal::concurrent_queue_base_v3::internal_push
              ((concurrent_queue_base_v3 *)((int)this + 0x150),&local_3c);
    local_4 = (uint)local_4._1_3_ << 8;
    if (local_3c != (undefined4 *)0x0) {
      piStack_30 = local_3c + 1;
      LOCK();
      iVar2 = *piStack_30;
      *piStack_30 = *piStack_30 + -1;
      UNLOCK();
      if ((iVar2 + -1 < 1) && (local_3c != (undefined4 *)0x0)) {
        (**(code **)*local_3c)();
      }
    }
  }
  else if (param_2 != (undefined4 *)0x0) {
    (**(code **)*param_2)();
  }
  ExceptionList = pvStack_c;
  return 0;
}


/* Mercury debug (Nub): Nub::processPacket( %s ): Not enough data for first request offset (%d bytes
   lef
   [String discovery] References: "Nub::processFilteredPacket"
   [String discovery] References: "Nub::processOrderedPacket" */

undefined4 __thiscall Mercury_Nub_14(void *this,uint *param_1,undefined4 *param_2,int param_3)

{
  byte *pbVar1;
  int iVar2;
  int iVar3;
  undefined8 uVar4;
  undefined4 uVar5;
  uint uVar6;
  int *piVar7;
  int iVar8;
  void *pvVar9;
  undefined4 *puVar10;
  undefined4 *puVar11;
  int *piVar12;
  void *this_00;
  undefined4 *extraout_ECX;
  undefined4 *extraout_ECX_00;
  undefined4 *extraout_ECX_01;
  undefined4 *extraout_ECX_02;
  undefined4 *extraout_ECX_03;
  int extraout_EDX;
  int extraout_EDX_00;
  void *local_84;
  int *local_80;
  undefined4 *local_7c;
  void *local_78;
  undefined4 local_74;
  int local_70;
  int *local_6c;
  int *piStack_68;
  int *local_64;
  int *local_60;
  int *local_5c;
  undefined1 *local_58;
  undefined1 *local_54;
  undefined1 *local_50;
  int local_4c;
  int local_48;
  undefined4 *local_44;
  undefined4 local_40;
  undefined4 local_3c [6];
  uint local_24;
  uint local_20;
  int local_1c;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  iVar3 = param_3;
  puVar10 = param_2;
  puStack_8 = &LAB_01793aed;
  local_c = ExceptionList;
  local_74 = 0;
  local_4 = 0;
  ExceptionList = &local_c;
  local_80 = this;
  if ((*(byte *)(param_2 + 0x15) & 1) != 0) {
    iVar2 = param_2[9];
    if (1 < iVar2 + -1) {
      ExceptionList = &local_c;
      param_2[10] = param_2[10] + 2;
      param_2[9] = iVar2 + -2;
      *(undefined2 *)(param_2 + 0xc) = *(undefined2 *)(iVar2 + 0x52 + (int)param_2);
      goto LAB_0157fd7a;
    }
    if (param_3 == 0) {
      ExceptionList = &local_c;
      Mercury__unknown_015847f0(param_1);
    }
    else {
      ExceptionList = &local_c;
      Mercury__unknown_0158bbb0(param_3);
    }
    _00___FRawStaticIndexBuffer__vfunc_0();
    local_6c = (int *)((int)this + 0xf8);
    LOCK();
    piStack_68 = (int *)*local_6c;
    *local_6c = *local_6c + 1;
    UNLOCK();
    goto LAB_0157fdf5;
  }
LAB_0157fd7a:
  if ((*(byte *)(param_2 + 0x15) & 0x20) == 0) {
    FUN_0158a4f0((int)param_2);
    pvVar9 = (void *)scalable_malloc();
    if (pvVar9 == (void *)0x0) {
      FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_4._0_1_ = 8;
    local_50 = pvVar9;
    this_00 = (void *)FUN_00418e30(0x74);
    local_4._0_1_ = 9;
    if (this_00 == (void *)0x0) {
      puVar11 = (undefined4 *)0x0;
    }
    else {
      local_58 = &stack0xffffff68;
      puVar11 = extraout_ECX_03;
      local_54 = this_00;
      Mercury__unknown_01576d50((int *)&stack0xffffff68,(int)&param_2,(int *)&param_2);
      local_4 = CONCAT31(local_4._1_3_,9);
      puVar11 = FUN_0157ab10(this_00,puVar11);
    }
    local_58 = &stack0xffffff64;
    local_74 = 1;
    local_54 = (undefined1 *)0x0;
    local_4 = 0xb;
    local_7c = FUN_0158d7d0(pvVar9,param_1,puVar11,iVar3 != 0);
    if (local_7c != (undefined4 *)0x0) {
      local_5c = local_7c + 1;
      LOCK();
      local_50 = (undefined1 *)*local_5c;
      *local_5c = *local_5c + 1;
      UNLOCK();
      puVar10 = param_2;
    }
    local_4 = 0xe;
    tbb::internal::concurrent_queue_base_v3::internal_push
              ((concurrent_queue_base_v3 *)((int)local_80 + 0x138),&local_7c);
    local_4 = CONCAT31(local_4._1_3_,0xd);
    if (local_7c != (undefined4 *)0x0) {
      local_80 = local_7c + 1;
      LOCK();
      local_70 = *local_80;
      *local_80 = *local_80 + -1;
      UNLOCK();
      puVar10 = param_2;
      if ((local_70 == 1 || local_70 + -1 < 0) && (local_7c != (undefined4 *)0x0)) {
        (**(code **)*local_7c)();
        puVar10 = param_2;
      }
    }
    local_74 = 0;
    local_4 = 0xffffffff;
    if (puVar10 != (undefined4 *)0x0) {
      local_80 = puVar10 + 1;
      LOCK();
      local_6c = (int *)*local_80;
      *local_80 = *local_80 + -1;
      UNLOCK();
      if (local_6c == (int *)0x1 || (int)local_6c + -1 < 0) {
        (**(code **)*param_2)();
      }
    }
LAB_01580601:
    uVar5 = 0;
  }
  else {
    iVar3 = param_2[9];
    if (iVar3 + -1 < 8) {
      if (param_3 == 0) {
        Mercury__unknown_015847f0(param_1);
      }
      else {
        Mercury__unknown_0158bbb0(param_3);
      }
      _00___FRawStaticIndexBuffer__vfunc_0();
      piStack_68 = (int *)((int)this + 0xf8);
      LOCK();
      local_64 = (int *)*piStack_68;
      *piStack_68 = *piStack_68 + 1;
      UNLOCK();
    }
    else if (iVar3 + -1 < 4) {
      Mercury__unknown_015847f0(param_1);
      _00___FRawStaticIndexBuffer__vfunc_0();
      local_64 = (int *)((int)this + 0xf8);
      LOCK();
      local_60 = (int *)*local_64;
      *local_64 = *local_64 + 1;
      UNLOCK();
    }
    else {
      param_2[10] = param_2[10] + 4;
      param_2[9] = iVar3 + -4;
      pbVar1 = (byte *)(param_2 + 0x13);
      param_2[0x14] = *(undefined4 *)(iVar3 + 0x50 + (int)param_2);
      uVar6 = Mercury__unknown_0157b410(param_2,(undefined4 *)pbVar1);
      if ((char)uVar6 == '\0') {
        Mercury__unknown_015847f0(param_1);
        _00___FRawStaticIndexBuffer__vfunc_0();
        local_60 = (int *)((int)local_80 + 0xf8);
        LOCK();
        local_5c = (int *)*local_60;
        *local_60 = *local_60 + 1;
        UNLOCK();
      }
      else {
        iVar3 = *(int *)pbVar1;
        iVar2 = puVar10[0x14];
        if (1 < (iVar2 - iVar3) + 1) {
          local_24 = *param_1;
          local_20 = param_1[1];
          local_7c = (undefined4 *)0x0;
          local_4c = 0;
          local_48 = 0;
          local_1c = iVar3;
          if (param_3 == 0) {
            puVar11 = (undefined4 *)
                      FUN_011d1710((void *)((int)local_80 + 0xac),
                                   (uint)(byte)(*(byte *)((int)param_1 + 5) ^
                                                *(byte *)((int)param_1 + 3) ^ *pbVar1));
            local_7c = puVar11;
            piVar7 = (int *)FUN_0157b9a0(puVar11,local_3c,&local_24);
            local_4c = *piVar7;
            local_40 = puVar11[1];
            local_48 = piVar7[1];
            local_44 = puVar11;
            uVar5 = FUN_00ead4a0(&local_4c,(int *)&local_44);
            if ((char)uVar5 != '\0') {
              iVar8 = FUN_00569a00(&local_4c);
              piVar7 = *(int **)(iVar8 + 0xc);
              goto LAB_0158000d;
            }
LAB_015800d7:
            pvVar9 = (void *)FUN_00418e30(0x14);
            local_4._0_1_ = 1;
            local_50 = pvVar9;
            if (pvVar9 == (void *)0x0) {
              puVar10 = (undefined4 *)0x0;
            }
            else {
              local_54 = &stack0xffffff68;
              puVar11 = extraout_ECX;
              Mercury__unknown_01576d50((int *)&stack0xffffff68,(int)&param_2,(int *)&param_2);
              uVar4 = rdtsc();
              local_4._0_1_ = 1;
              puVar10 = FUN_0157e2c0(pvVar9,puVar10[0x14],iVar2 - iVar3,(int)uVar4,
                                     (int)((ulonglong)uVar4 >> 0x20),puVar11);
            }
            local_4 = (uint)local_4._1_3_ << 8;
            if (param_3 == 0) {
              puVar11 = FUN_0157cf60(local_7c,&local_24);
              *puVar11 = puVar10;
            }
            else {
              *(undefined4 **)(param_3 + 0x124) = puVar10;
            }
          }
          else {
            piVar7 = *(int **)(param_3 + 0x124);
LAB_0158000d:
            if (piVar7 == (int *)0x0) {
              if ((param_3 != 0) && (puVar10[0x11] != *(int *)pbVar1)) {
                Mercury__unknown_0158bbb0(param_3);
                _00___FRawStaticIndexBuffer__vfunc_0();
                goto LAB_0157ff43;
              }
              goto LAB_015800d7;
            }
            if (((*(byte *)(piVar7[4] + 0x54) & 0x10) == 0) ||
               (uVar5 = FUN_0157b120((int)piVar7), (char)uVar5 != '\0')) {
              if (param_3 == 0) {
                Mercury__unknown_015847f0(param_1);
              }
              else {
                Mercury__unknown_0158bbb0(param_3);
              }
              _00___FRawStaticIndexBuffer__vfunc_0();
              FUN_01579080(piVar7 + 4,0);
              Mercury__unknown_0157f050((int)piVar7);
                    /* WARNING: Subroutine does not return */
              scalable_free();
            }
            if (*piVar7 == puVar10[0x14]) {
              uVar4 = rdtsc();
              piVar7[2] = (int)uVar4;
              piVar7[3] = (int)((ulonglong)uVar4 >> 0x20);
              local_78 = (void *)0x0;
              local_84 = (void *)0x0;
              local_4 = CONCAT31((int3)((uint)local_4 >> 8),4);
              Mercury__unknown_01576d80(&local_84,piVar7 + 4);
              puVar11 = extraout_ECX_00;
              iVar3 = extraout_EDX;
              pvVar9 = local_84;
              while (local_84 = pvVar9, pvVar9 != (void *)0x0) {
                puVar11 = *(undefined4 **)((int)pvVar9 + 0x44);
                if (puVar11 == (undefined4 *)puVar10[0x11]) {
                  if (param_3 == 0) {
                    Mercury__unknown_015847f0(param_1);
                  }
                  else {
                    Mercury__unknown_0158bbb0(param_3);
                  }
                  _00___FRawStaticIndexBuffer__vfunc_0();
                  local_4._0_1_ = 3;
                  Mercury__unknown_0157df90((int *)&local_84);
                  local_4 = (uint)local_4._1_3_ << 8;
                  Mercury__unknown_0157df90((int *)&local_78);
                  goto LAB_0158015b;
                }
                if (0x8000000 < ((int)puVar10[0x11] - (int)puVar11 & 0xfffffffU)) break;
                Mercury__unknown_01576d80(&local_78,(int *)&local_84);
                piVar12 = Mercury__unknown_0158a850(pvVar9,(int *)&local_58);
                local_4._0_1_ = 5;
                Mercury__unknown_01576d80(&local_84,piVar12);
                local_4 = CONCAT31(local_4._1_3_,4);
                Mercury__unknown_0157df90((int *)&local_58);
                puVar11 = extraout_ECX_01;
                iVar3 = extraout_EDX_00;
                pvVar9 = local_84;
              }
              local_50 = &stack0xffffff68;
              Mercury__unknown_01576d50((int *)&stack0xffffff68,iVar3,(int *)&local_84);
              local_4._1_3_ = (uint3)((uint)local_4 >> 8);
              local_4._0_1_ = 4;
              Mercury__unknown_015792f0(puVar10,puVar11);
              pvVar9 = local_78;
              if (local_78 == (void *)0x0) {
                Mercury__unknown_01576d80(piVar7 + 4,(int *)&param_2);
              }
              else {
                local_50 = &stack0xffffff68;
                puVar10 = extraout_ECX_02;
                Mercury__unknown_01576d50((int *)&stack0xffffff68,(int)&param_2,(int *)&param_2);
                local_4._0_1_ = 4;
                Mercury__unknown_015792f0(pvVar9,puVar10);
              }
              piVar7[1] = piVar7[1] + -1;
              if (piVar7[1] < 1) {
                Mercury__unknown_01576d80(&param_2,piVar7 + 4);
                Mercury__unknown_0157f050((int)piVar7);
                    /* WARNING: Subroutine does not return */
                scalable_free();
              }
              local_4._0_1_ = 3;
              Mercury__unknown_0157df90((int *)&local_84);
              local_4 = (uint)local_4._1_3_ << 8;
              Mercury__unknown_0157df90((int *)&local_78);
              local_4 = 0xffffffff;
              Mercury__unknown_0157df90((int *)&param_2);
              goto LAB_01580601;
            }
            if ((*(byte *)(puVar10 + 0x15) & 0x10) != 0) {
              if (param_3 == 0) {
                Mercury__unknown_015847f0(param_1);
              }
              else {
                Mercury__unknown_0158bbb0(param_3);
              }
              _00___FRawStaticIndexBuffer__vfunc_0();
              goto LAB_0157ff43;
            }
            if (param_3 == 0) {
              Mercury__unknown_015847f0(param_1);
            }
            else {
              Mercury__unknown_0158bbb0(param_3);
            }
            _00___FRawStaticIndexBuffer__vfunc_0();
          }
LAB_0158015b:
          local_4 = 0xffffffff;
          Mercury__unknown_0157df90((int *)&param_2);
          goto LAB_01580601;
        }
        if (param_3 == 0) {
          Mercury__unknown_015847f0(param_1);
        }
        else {
          Mercury__unknown_0158bbb0(param_3);
        }
        _00___FRawStaticIndexBuffer__vfunc_0();
LAB_0157ff43:
        Mercury__unknown_0157c020((int *)((int)local_80 + 0xf8));
      }
    }
LAB_0157fdf5:
    local_4 = 0xffffffff;
    Mercury__unknown_0157df90((int *)&param_2);
    uVar5 = 0xfffffffc;
  }
  ExceptionList = local_c;
  return uVar5;
}


/* WARNING: Removing unreachable block (ram,0x01580880) */
/* WARNING: Removing unreachable block (ram,0x015808c3) */
/* WARNING: Removing unreachable block (ram,0x015808ea) */
/* WARNING: Removing unreachable block (ram,0x015808f2) */
/* Mercury debug (Nub): Nub::processFilteredPacket( %s ): received packet with bad flags %x
    */

int __thiscall Mercury_Nub_12(void *this,uint *param_1,undefined4 *param_2)

{
  uint *puVar1;
  undefined1 *puVar2;
  bool bVar3;
  ushort uVar4;
  undefined4 *puVar5;
  int iVar6;
  uint uVar7;
  undefined4 uVar8;
  undefined4 *extraout_ECX;
  int extraout_EDX;
  int iVar9;
  size_t _Size;
  uint uVar10;
  int *piVar11;
  int *piVar12;
  undefined4 *puVar13;
  int iStack_6c;
  char cStack_68;
  undefined4 auStack_64 [2];
  int iStack_5c;
  int iStack_58;
  int iStack_54;
  int iStack_50;
  int iStack_4c;
  int local_48;
  int *local_44;
  int local_40;
  int local_3c;
  int local_38;
  uint uStack_30;
  int iStack_2c;
  void *local_28;
  undefined4 *local_24;
  int iStack_20;
  undefined4 *local_1c;
  undefined1 local_16;
  byte local_15;
  undefined1 *local_14;
  void *pvStack_10;
  undefined1 *puStack_c;
  uint local_8;
  
  puVar1 = param_1;
  puStack_c = &LAB_01793b8b;
  pvStack_10 = ExceptionList;
  local_14 = &stack0xffffff88;
  local_8 = 0;
  local_15 = *(byte *)(param_2 + 0x15);
  local_28 = this;
  if ((int)(param_2[10] + param_2[9]) < 2) {
    ExceptionList = &pvStack_10;
    local_14 = &stack0xffffff88;
    Mercury__unknown_015847f0(param_1);
    _00___FRawStaticIndexBuffer__vfunc_0();
    piVar11 = (int *)((int)this + 0xf8);
    LOCK();
    local_38 = *piVar11;
    *piVar11 = *piVar11 + 1;
    UNLOCK();
  }
  else if ((local_15 & 2) == 0) {
    local_1c = (undefined4 *)0x0;
    if ((char)local_15 < '\0') {
      ExceptionList = &pvStack_10;
      local_14 = &stack0xffffff88;
      Mercury__unknown_015847f0(param_1);
      _00___FRawStaticIndexBuffer__vfunc_0();
      piVar11 = (int *)((int)this + 0xf8);
      LOCK();
      iStack_4c = *piVar11;
      *piVar11 = *piVar11 + 1;
      UNLOCK();
      local_8 = 0xffffffff;
      if (param_2 == (undefined4 *)0x0) {
        ExceptionList = pvStack_10;
        return -4;
      }
      piVar11 = param_2 + 1;
      LOCK();
      iStack_2c = *piVar11;
      *piVar11 = *piVar11 + -1;
      UNLOCK();
      if (iStack_2c != 1 && -1 < iStack_2c + -1) {
        ExceptionList = pvStack_10;
        return -4;
      }
      if (param_2 == (undefined4 *)0x0) {
        ExceptionList = pvStack_10;
        return -4;
      }
      (**(code **)*param_2)();
      ExceptionList = pvStack_10;
      return -4;
    }
    ExceptionList = &pvStack_10;
    puVar2 = &stack0xffffff88;
    if ((local_15 & 4) != 0) {
      param_1._3_1_ = local_15 & 8;
      ExceptionList = &pvStack_10;
      puVar2 = &stack0xffffff88;
      if ((local_15 & 8) != 0) {
        ExceptionList = &pvStack_10;
        local_1c = (undefined4 *)Mercury__unknown_0157c7b0(this,puVar1);
        puVar2 = local_14;
      }
      local_14 = puVar2;
      iVar6 = param_2[9];
      iVar9 = iVar6 + -1;
      if (iVar9 < 1) {
        Mercury__unknown_015847f0(puVar1);
        _00___FRawStaticIndexBuffer__vfunc_0();
        piVar11 = (int *)((int)this + 0xf8);
        LOCK();
        iStack_50 = *piVar11;
        *piVar11 = *piVar11 + 1;
        UNLOCK();
        goto LAB_015810c0;
      }
      param_2[9] = iVar9;
      param_2[10] = param_2[10] + 1;
      *(undefined1 *)(param_2 + 0xe) = *(undefined1 *)(iVar6 + 0x53 + (int)param_2);
      if (*(byte *)(param_2 + 0xe) == 0) {
        Mercury__unknown_015847f0(puVar1);
        _00___FRawStaticIndexBuffer__vfunc_0();
        piVar11 = (int *)((int)this + 0xf8);
        LOCK();
        iStack_54 = *piVar11;
        *piVar11 = *piVar11 + 1;
        UNLOCK();
        goto LAB_015810c0;
      }
      uVar7 = (uint)*(byte *)(param_2 + 0xe);
      if (param_2[9] + -1 < (int)(uVar7 * 4)) {
        Mercury__unknown_015847f0(puVar1);
        _00___FRawStaticIndexBuffer__vfunc_0();
        Mercury__unknown_0157c020((int *)((int)this + 0xf8));
        goto LAB_015810c0;
      }
      puVar2 = local_14;
      if (local_1c == (undefined4 *)0x0) {
        if (param_1._3_1_ == 0) {
          if (*(char *)(param_2 + 0xe) != '\0') {
            uVar7 = Mercury__unknown_0157b410(param_2,auStack_64);
            if ((char)uVar7 == '\0') {
              Mercury__unknown_015847f0(puVar1);
LAB_01580cc7:
              _00___FRawStaticIndexBuffer__vfunc_0();
              Mercury__unknown_0157c020((int *)((int)this + 0xf8));
            }
            else {
              Mercury__unknown_015847f0(puVar1);
              _00___FRawStaticIndexBuffer__vfunc_0();
              Mercury__unknown_0157c020((int *)((int)this + 0xf8));
            }
            goto LAB_015810c0;
          }
        }
        else {
          param_2[9] = param_2[9] + uVar7 * -4;
          Mercury__unknown_015847f0(puVar1);
          _00___FRawStaticIndexBuffer__vfunc_0();
          puVar2 = local_14;
        }
      }
      else {
        uVar10 = 0;
        if (uVar7 != 0) {
          do {
            uVar7 = Mercury__unknown_0157b410(param_2,&uStack_30);
            if ((char)uVar7 == '\0') {
              Mercury__unknown_015847f0(puVar1);
              goto LAB_01580cc7;
            }
            uVar8 = FUN_0158cac0(local_1c,uStack_30);
            if ((char)uVar8 == '\0') {
              Mercury__unknown_015847f0(puVar1);
              goto LAB_01580cc7;
            }
            uVar10 = uVar10 + 1;
            puVar2 = local_14;
          } while (uVar10 < *(byte *)(param_2 + 0xe));
        }
      }
    }
    local_14 = puVar2;
    if ((local_15 & 0x40) == 0) {
      Mercury__unknown_015847f0(puVar1);
      _00___FRawStaticIndexBuffer__vfunc_0();
      piVar11 = (int *)((int)this + 0xf8);
      LOCK();
      iStack_5c = *piVar11;
      *piVar11 = *piVar11 + 1;
      UNLOCK();
    }
    else {
      iVar6 = param_2[9];
      if (iVar6 + -1 < 4) {
        Mercury__unknown_015847f0(puVar1);
        _00___FRawStaticIndexBuffer__vfunc_0();
        piVar11 = (int *)((int)this + 0xf8);
        LOCK();
        iStack_58 = *piVar11;
        *piVar11 = *piVar11 + 1;
        UNLOCK();
      }
      else {
        param_2[9] = iVar6 + -4;
        param_2[10] = param_2[10] + 4;
        param_2[0x11] = *(undefined4 *)(iVar6 + 0x50 + (int)param_2);
        uVar7 = param_2[0x11];
        if (uVar7 == 0x10000000) {
          Mercury__unknown_015847f0(puVar1);
          _00___FRawStaticIndexBuffer__vfunc_0();
          Mercury__unknown_0157c020((int *)((int)this + 0xf8));
        }
        else if ((uVar7 & 0xfffffff) == uVar7) {
          if ((local_15 & 0x10) == 0) {
            if ((local_15 & 8) != 0) {
              if ((local_1c == (undefined4 *)0x0) &&
                 (local_1c = (undefined4 *)Mercury__unknown_0157c7b0(this,puVar1),
                 local_1c == (undefined4 *)0x0)) {
                _00___FRawStaticIndexBuffer__vfunc_0();
              }
              else {
                puVar5 = local_1c;
                bVar3 = FUN_0158bb50(local_1c,param_2[0x11]);
                if (bVar3) {
                  Mercury__unknown_0158bbb0((int)puVar5);
                  _00___FRawStaticIndexBuffer__vfunc_0();
                  *(int *)((int)this + 0x11c) = *(int *)((int)this + 0x11c) + 1;
                  local_8 = 0xffffffff;
                  Mercury__unknown_0157df90((int *)&param_2);
                  ExceptionList = pvStack_10;
                  return -0xb;
                }
              }
            }
LAB_01580f25:
            iVar6 = 0;
            while( true ) {
              Mercury__unknown_0158a850(param_2,&iStack_20);
              local_8._1_3_ = (uint3)(local_8 >> 8);
              local_8._0_1_ = 8;
              Mercury__unknown_015792f0(param_2,(undefined4 *)0x0);
              puVar5 = local_1c;
              puVar13 = local_1c;
              Mercury__unknown_01576d50((int *)&stack0xffffff80,(int)&param_2,(int *)&param_2);
              local_8._0_1_ = 8;
              iVar9 = Mercury_Nub_14(this,puVar1,puVar5,(int)puVar13);
              if (iVar6 == 0) {
                iVar6 = iVar9;
              }
              piVar11 = Mercury__unknown_01576d80(&param_2,&iStack_20);
              if (*piVar11 == 0) break;
              local_8 = (uint)local_8._1_3_ << 8;
              Mercury__unknown_0157df90(&iStack_20);
            }
            local_8 = (uint)local_8._1_3_ << 8;
            Mercury__unknown_0157df90(&iStack_20);
            local_8 = 0xffffffff;
            Mercury__unknown_0157df90((int *)&param_2);
            ExceptionList = pvStack_10;
            return iVar6;
          }
          if ((local_15 & 8) != 0) {
            if ((local_1c == (undefined4 *)0x0) &&
               (local_1c = (undefined4 *)Mercury__unknown_0157c7b0(this,puVar1),
               local_1c == (undefined4 *)0x0)) {
              Mercury__unknown_015847f0(puVar1);
              _00___FRawStaticIndexBuffer__vfunc_0();
            }
            else {
              piVar11 = (int *)param_2[0x11];
              piVar12 = piVar11;
              Mercury__unknown_01576d50((int *)&stack0xffffff7c,(int)&param_2,(int *)&param_2);
              local_8 = local_8 & 0xffffff00;
              UnAckedHandler_queueAckForPacket(local_1c,&iStack_6c,piVar11,piVar12);
              local_8._0_1_ = 7;
              if (iStack_6c != 0) {
                local_8 = (uint)local_8._1_3_ << 8;
                InterfaceElement__unknown_0158c100(&iStack_6c);
                goto LAB_01580f25;
              }
              if (cStack_68 == '\0') {
                Mercury__unknown_0157c020((int *)((int)this + 0xf8));
                local_8 = (uint)local_8._1_3_ << 8;
                InterfaceElement__unknown_0158c100(&iStack_6c);
                goto LAB_015810c0;
              }
              local_8 = (uint)local_8._1_3_ << 8;
              InterfaceElement__unknown_0158c100(&iStack_6c);
            }
            local_8 = 0xffffffff;
            Mercury__unknown_0157df90((int *)&param_2);
            ExceptionList = pvStack_10;
            return 0;
          }
          Mercury__unknown_015847f0(puVar1);
          _00___FRawStaticIndexBuffer__vfunc_0();
          Mercury__unknown_0157c020((int *)((int)this + 0xf8));
        }
        else {
          Mercury__unknown_015847f0(puVar1);
          _00___FRawStaticIndexBuffer__vfunc_0();
          Mercury__unknown_0157c020((int *)((int)this + 0xf8));
        }
      }
    }
  }
  else {
    local_16 = 0;
    iVar6 = param_2[9];
    if (iVar6 + -1 < 2) {
      ExceptionList = &pvStack_10;
      local_14 = &stack0xffffff88;
      Mercury__unknown_015847f0(param_1);
      _00___FRawStaticIndexBuffer__vfunc_0();
      piVar11 = (int *)((int)this + 0xf8);
      LOCK();
      local_3c = *piVar11;
      *piVar11 = *piVar11 + 1;
      UNLOCK();
    }
    else {
      ExceptionList = &pvStack_10;
      param_2[9] = iVar6 + -2;
      param_2[10] = param_2[10] + 2;
      uVar4 = *(ushort *)((int)param_2 + iVar6 + 0x52);
      local_16 = (short)uVar4 < 0;
      if ((bool)local_16) {
        uVar4 = ~uVar4;
      }
      _Size = (size_t)(short)uVar4;
      if ((int)_Size <= param_2[9] + -1) {
        local_14 = &stack0xffffff88;
        local_44 = (int *)FUN_00418e30(0x614);
        local_8._0_1_ = 1;
        if (local_44 == (int *)0x0) {
          puVar5 = (undefined4 *)0x0;
        }
        else {
          puVar5 = FUN_0158a260(local_44);
        }
        if (puVar5 != (undefined4 *)0x0) {
          local_44 = puVar5 + 1;
          LOCK();
          local_48 = *local_44;
          *local_44 = *local_44 + 1;
          UNLOCK();
        }
        local_8._0_1_ = 2;
        param_2[9] = param_2[9] - _Size;
        local_24 = puVar5;
        memcpy(puVar5 + 0x15,(void *)(param_2[9] + 0x54 + (int)param_2),_Size);
        puVar5[9] = _Size;
        local_8._0_1_ = 3;
        local_44 = (int *)&stack0xffffff84;
        puVar5 = extraout_ECX;
        Mercury__unknown_01576d50((int *)&stack0xffffff84,extraout_EDX,(int *)&local_24);
        local_8 = CONCAT31(local_8._1_3_,3);
        Mercury_Nub_12(local_28,param_1,puVar5);
        local_8 = 2;
        iVar6 = Mercury_Nub_5();
        return iVar6;
      }
      local_14 = &stack0xffffff88;
      Mercury__unknown_015847f0(param_1);
      _00___FRawStaticIndexBuffer__vfunc_0();
      piVar11 = (int *)((int)this + 0xf8);
      LOCK();
      local_40 = *piVar11;
      *piVar11 = *piVar11 + 1;
      UNLOCK();
    }
  }
LAB_015810c0:
  local_8 = 0xffffffff;
  Mercury__unknown_0157df90((int *)&param_2);
  ExceptionList = pvStack_10;
  return -4;
}


/* Mercury debug (Nub): Nub::processFilteredPacket( %s ): Got an exception whilst processing
   piggyback p */

undefined * Mercury_Nub_13(void)

{
  int unaff_EBP;
  
  Mercury__unknown_00de15e0(*(int *)(*(int *)(unaff_EBP + -0x5c) + 4));
  Mercury__unknown_015847f0(*(void **)(unaff_EBP + 8));
  _00___FRawStaticIndexBuffer__vfunc_0();
  *(undefined4 *)(unaff_EBP + -4) = 2;
  return &DAT_01580ad1;
}


/* Mercury error in Nub: Nub::processFilteredPacket( %s ): delResendTimer() failed for #%d
    */

int Mercury_Nub_5(void)

{
  byte bVar1;
  void *this;
  bool bVar2;
  ushort uVar3;
  undefined4 *puVar4;
  int iVar5;
  undefined4 uVar6;
  uint uVar7;
  int *piVar8;
  undefined4 *extraout_ECX;
  void *pvVar9;
  int extraout_EDX;
  int iVar10;
  uint *unaff_EBX;
  int unaff_EBP;
  size_t _Size;
  uint uVar11;
  int *piVar12;
  undefined4 *puVar13;
  
  *(undefined1 *)(unaff_EBP + -4) = 0;
  Mercury__unknown_0157df90((int *)(unaff_EBP + -0x20));
  pvVar9 = *(void **)(unaff_EBP + 0xc);
  this = *(void **)(unaff_EBP + -0x24);
  bVar1 = *(byte *)(unaff_EBP + -0x11);
  if (*(char *)(unaff_EBP + -0x12) == '\0') {
    iVar5 = *(int *)((int)pvVar9 + 0x24);
    if (iVar5 + -1 < 2) {
      Mercury__unknown_015847f0(unaff_EBX);
      _00___FRawStaticIndexBuffer__vfunc_0();
      *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
      piVar8 = *(int **)(unaff_EBP + 8);
      LOCK();
      iVar5 = *piVar8;
      *piVar8 = *piVar8 + 1;
      UNLOCK();
      *(int *)(unaff_EBP + -0x38) = iVar5;
    }
    else {
      *(int *)((int)pvVar9 + 0x24) = iVar5 + -2;
      *(int *)((int)pvVar9 + 0x28) = *(int *)((int)pvVar9 + 0x28) + 2;
      uVar3 = *(ushort *)((int)pvVar9 + iVar5 + 0x52);
      if ((short)uVar3 < 0) {
        uVar3 = ~uVar3;
        *(undefined1 *)(unaff_EBP + -0x12) = 1;
      }
      _Size = (size_t)(short)uVar3;
      if ((int)_Size <= *(int *)(*(int *)(unaff_EBP + 0xc) + 0x24) + -1) {
        puVar4 = (undefined4 *)FUN_00418e30(0x614);
        *(undefined4 **)(unaff_EBP + -0x40) = puVar4;
        *(undefined1 *)(unaff_EBP + -4) = 1;
        if (puVar4 == (undefined4 *)0x0) {
          puVar4 = (undefined4 *)0x0;
        }
        else {
          puVar4 = FUN_0158a260(puVar4);
        }
        *(undefined1 *)(unaff_EBP + -4) = 0;
        *(undefined4 **)(unaff_EBP + -0x20) = puVar4;
        if (puVar4 != (undefined4 *)0x0) {
          *(undefined4 **)(unaff_EBP + -0x40) = puVar4 + 1;
          piVar8 = *(int **)(unaff_EBP + -0x40);
          LOCK();
          iVar5 = *piVar8;
          *piVar8 = *piVar8 + 1;
          UNLOCK();
          *(int *)(unaff_EBP + -0x44) = iVar5;
        }
        *(undefined1 *)(unaff_EBP + -4) = 2;
        piVar8 = (int *)(*(int *)(unaff_EBP + 0xc) + 0x24);
        *piVar8 = *piVar8 - _Size;
        memcpy(puVar4 + 0x15,
               (void *)(*(int *)(*(int *)(unaff_EBP + 0xc) + 0x24) + 0x54 +
                       *(int *)(unaff_EBP + 0xc)),_Size);
        puVar4[9] = _Size;
        *(undefined1 *)(unaff_EBP + -4) = 3;
        *(undefined1 **)(unaff_EBP + -0x40) = &stack0xfffffffc;
        puVar4 = extraout_ECX;
        Mercury__unknown_01576d50((int *)&stack0xfffffffc,extraout_EDX,(int *)(unaff_EBP + -0x20));
        *(undefined1 *)(unaff_EBP + -4) = 4;
        *(undefined1 *)(unaff_EBP + -4) = 3;
        Mercury_Nub_12(*(void **)(unaff_EBP + -0x24),unaff_EBX,puVar4);
        *(undefined4 *)(unaff_EBP + -4) = 2;
        iVar5 = Mercury_Nub_5();
        return iVar5;
      }
      Mercury__unknown_015847f0(unaff_EBX);
      _00___FRawStaticIndexBuffer__vfunc_0();
      *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
      piVar8 = *(int **)(unaff_EBP + 8);
      LOCK();
      iVar5 = *piVar8;
      *piVar8 = *piVar8 + 1;
      UNLOCK();
      *(int *)(unaff_EBP + -0x3c) = iVar5;
    }
  }
  else {
    *(undefined4 *)(unaff_EBP + -0x18) = 0;
    if ((char)bVar1 < '\0') {
      Mercury__unknown_015847f0(unaff_EBX);
      _00___FRawStaticIndexBuffer__vfunc_0();
      *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
      piVar8 = *(int **)(unaff_EBP + 8);
      LOCK();
      iVar5 = *piVar8;
      *piVar8 = *piVar8 + 1;
      UNLOCK();
      *(int *)(unaff_EBP + -0x48) = iVar5;
      *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
      puVar4 = *(undefined4 **)(unaff_EBP + 0xc);
      if (puVar4 != (undefined4 *)0x0) {
        *(undefined4 **)(unaff_EBP + 8) = puVar4 + 1;
        piVar8 = *(int **)(unaff_EBP + 8);
        LOCK();
        iVar5 = *piVar8;
        *piVar8 = *piVar8 + -1;
        UNLOCK();
        *(int *)(unaff_EBP + -0x28) = iVar5;
        if ((*(int *)(unaff_EBP + -0x28) == 1 || *(int *)(unaff_EBP + -0x28) + -1 < 0) &&
           (puVar4 != (undefined4 *)0x0)) {
          (**(code **)*puVar4)();
        }
      }
      goto LAB_015810cf;
    }
    if ((bVar1 & 4) == 0) goto LAB_01580d6e;
    *(byte *)(unaff_EBP + 0xb) = bVar1 & 8;
    if ((bVar1 & 8) != 0) {
      uVar6 = Mercury__unknown_0157c7b0(this,unaff_EBX);
      *(undefined4 *)(unaff_EBP + -0x18) = uVar6;
      pvVar9 = *(void **)(unaff_EBP + 0xc);
    }
    iVar5 = *(int *)((int)pvVar9 + 0x24);
    iVar10 = iVar5 + -1;
    if (iVar10 < 1) {
      Mercury__unknown_015847f0(unaff_EBX);
      _00___FRawStaticIndexBuffer__vfunc_0();
      *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
      piVar8 = *(int **)(unaff_EBP + 8);
      LOCK();
      iVar5 = *piVar8;
      *piVar8 = *piVar8 + 1;
      UNLOCK();
      *(int *)(unaff_EBP + -0x4c) = iVar5;
    }
    else {
      *(int *)((int)pvVar9 + 0x24) = iVar10;
      *(int *)((int)pvVar9 + 0x28) = *(int *)((int)pvVar9 + 0x28) + 1;
      *(undefined1 *)((int)pvVar9 + 0x38) = *(undefined1 *)(iVar5 + 0x53 + (int)pvVar9);
      pvVar9 = *(void **)(unaff_EBP + 0xc);
      if (*(byte *)((int)pvVar9 + 0x38) == 0) {
        Mercury__unknown_015847f0(unaff_EBX);
        _00___FRawStaticIndexBuffer__vfunc_0();
        *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
        piVar8 = *(int **)(unaff_EBP + 8);
        LOCK();
        iVar5 = *piVar8;
        *piVar8 = *piVar8 + 1;
        UNLOCK();
        *(int *)(unaff_EBP + -0x50) = iVar5;
      }
      else {
        uVar7 = (uint)*(byte *)((int)pvVar9 + 0x38);
        if (*(int *)((int)pvVar9 + 0x24) + -1 < (int)(uVar7 * 4)) {
          Mercury__unknown_015847f0(unaff_EBX);
          _00___FRawStaticIndexBuffer__vfunc_0();
          Mercury__unknown_0157c020((int *)((int)this + 0xf8));
        }
        else {
          if (*(int *)(unaff_EBP + -0x18) == 0) {
            if (*(char *)(unaff_EBP + 0xb) == '\0') {
              if (*(char *)((int)pvVar9 + 0x38) != '\0') {
                uVar7 = Mercury__unknown_0157b410(pvVar9,(undefined4 *)(unaff_EBP + -0x60));
                if ((char)uVar7 == '\0') {
                  Mercury__unknown_015847f0(unaff_EBX);
LAB_01580cc7:
                  _00___FRawStaticIndexBuffer__vfunc_0();
                  Mercury__unknown_0157c020((int *)((int)this + 0xf8));
                }
                else {
                  Mercury__unknown_015847f0(unaff_EBX);
                  _00___FRawStaticIndexBuffer__vfunc_0();
                  Mercury__unknown_0157c020((int *)((int)this + 0xf8));
                }
                goto LAB_015810c0;
              }
            }
            else {
              *(int *)((int)pvVar9 + 0x24) = *(int *)((int)pvVar9 + 0x24) + uVar7 * -4;
              Mercury__unknown_015847f0(unaff_EBX);
              _00___FRawStaticIndexBuffer__vfunc_0();
              pvVar9 = *(void **)(unaff_EBP + 0xc);
            }
          }
          else {
            uVar11 = 0;
            if (uVar7 != 0) {
              do {
                uVar7 = Mercury__unknown_0157b410(pvVar9,(undefined4 *)(unaff_EBP + -0x2c));
                if ((char)uVar7 == '\0') {
                  Mercury__unknown_015847f0(unaff_EBX);
                  goto LAB_01580cc7;
                }
                uVar6 = FUN_0158cac0(*(void **)(unaff_EBP + -0x18),*(uint *)(unaff_EBP + -0x2c));
                if ((char)uVar6 == '\0') {
                  Mercury__unknown_015847f0(unaff_EBX);
                  goto LAB_01580cc7;
                }
                uVar11 = uVar11 + 1;
                pvVar9 = *(void **)(unaff_EBP + 0xc);
              } while (uVar11 < *(byte *)((int)pvVar9 + 0x38));
            }
          }
LAB_01580d6e:
          if ((*(byte *)(unaff_EBP + -0x11) & 0x40) == 0) {
            Mercury__unknown_015847f0(unaff_EBX);
            _00___FRawStaticIndexBuffer__vfunc_0();
            *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
            piVar8 = *(int **)(unaff_EBP + 8);
            LOCK();
            iVar5 = *piVar8;
            *piVar8 = *piVar8 + 1;
            UNLOCK();
            *(int *)(unaff_EBP + -0x58) = iVar5;
          }
          else {
            *(void **)(unaff_EBP + 8) = pvVar9;
            iVar5 = *(int *)((int)pvVar9 + 0x24);
            if (iVar5 + -1 < 4) {
              Mercury__unknown_015847f0(unaff_EBX);
              _00___FRawStaticIndexBuffer__vfunc_0();
              *(int *)(unaff_EBP + 8) = (int)this + 0xf8;
              piVar8 = *(int **)(unaff_EBP + 8);
              LOCK();
              iVar5 = *piVar8;
              *piVar8 = *piVar8 + 1;
              UNLOCK();
              *(int *)(unaff_EBP + -0x54) = iVar5;
            }
            else {
              *(int *)((int)pvVar9 + 0x24) = iVar5 + -4;
              *(int *)((int)pvVar9 + 0x28) = *(int *)((int)pvVar9 + 0x28) + 4;
              *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x44) =
                   *(undefined4 *)(iVar5 + 0x50 + (int)pvVar9);
              uVar7 = *(uint *)(*(int *)(unaff_EBP + 0xc) + 0x44);
              if (uVar7 == 0x10000000) {
                Mercury__unknown_015847f0(unaff_EBX);
                _00___FRawStaticIndexBuffer__vfunc_0();
                Mercury__unknown_0157c020((int *)((int)this + 0xf8));
              }
              else if ((uVar7 & 0xfffffff) == uVar7) {
                bVar1 = *(byte *)(unaff_EBP + -0x11);
                if ((bVar1 & 0x10) == 0) {
                  if ((bVar1 & 8) != 0) {
                    if (*(int *)(unaff_EBP + -0x18) == 0) {
                      iVar5 = Mercury__unknown_0157c7b0(this,unaff_EBX);
                      *(int *)(unaff_EBP + -0x18) = iVar5;
                      if (iVar5 == 0) {
                        _00___FRawStaticIndexBuffer__vfunc_0();
                        goto LAB_01580f25;
                      }
                    }
                    pvVar9 = *(void **)(unaff_EBP + -0x18);
                    bVar2 = FUN_0158bb50(pvVar9,*(uint *)(*(int *)(unaff_EBP + 0xc) + 0x44));
                    if (bVar2) {
                      Mercury__unknown_0158bbb0((int)pvVar9);
                      _00___FRawStaticIndexBuffer__vfunc_0();
                      *(int *)((int)this + 0x11c) = *(int *)((int)this + 0x11c) + 1;
                      *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
                      Mercury__unknown_0157df90((int *)(unaff_EBP + 0xc));
                      ExceptionList = *(void **)(unaff_EBP + -0xc);
                      return -0xb;
                    }
                  }
LAB_01580f25:
                  iVar5 = 0;
                  while( true ) {
                    Mercury__unknown_0158a850
                              (*(void **)(unaff_EBP + 0xc),(int *)(unaff_EBP + -0x1c));
                    *(undefined1 *)(unaff_EBP + -4) = 8;
                    *(undefined1 **)(unaff_EBP + 8) = &stack0xfffffffc;
                    *(undefined1 *)(unaff_EBP + -4) = 9;
                    *(undefined1 *)(unaff_EBP + -4) = 8;
                    Mercury__unknown_015792f0(*(void **)(unaff_EBP + 0xc),(undefined4 *)0x0);
                    puVar4 = *(undefined4 **)(unaff_EBP + -0x18);
                    *(undefined1 **)(unaff_EBP + 8) = &stack0xfffffff8;
                    puVar13 = puVar4;
                    Mercury__unknown_01576d50
                              ((int *)&stack0xfffffff8,unaff_EBP + 0xc,(int *)(unaff_EBP + 0xc));
                    *(undefined1 *)(unaff_EBP + -4) = 10;
                    *(undefined1 *)(unaff_EBP + -4) = 8;
                    iVar10 = Mercury_Nub_14(this,unaff_EBX,puVar4,(int)puVar13);
                    if (iVar5 == 0) {
                      iVar5 = iVar10;
                    }
                    piVar8 = Mercury__unknown_01576d80
                                       ((void *)(unaff_EBP + 0xc),(int *)(unaff_EBP + -0x1c));
                    if (*piVar8 == 0) break;
                    *(undefined1 *)(unaff_EBP + -4) = 0;
                    Mercury__unknown_0157df90((int *)(unaff_EBP + -0x1c));
                  }
                  *(undefined1 *)(unaff_EBP + -4) = 0;
                  Mercury__unknown_0157df90((int *)(unaff_EBP + -0x1c));
                  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
                  Mercury__unknown_0157df90((int *)(unaff_EBP + 0xc));
                  ExceptionList = *(void **)(unaff_EBP + -0xc);
                  return iVar5;
                }
                if ((bVar1 & 8) == 0) {
                  Mercury__unknown_015847f0(unaff_EBX);
                  _00___FRawStaticIndexBuffer__vfunc_0();
                  Mercury__unknown_0157c020((int *)((int)this + 0xf8));
                }
                else {
                  if (*(int *)(unaff_EBP + -0x18) == 0) {
                    iVar5 = Mercury__unknown_0157c7b0(this,unaff_EBX);
                    *(int *)(unaff_EBP + -0x18) = iVar5;
                    if (iVar5 == 0) {
                      Mercury__unknown_015847f0(unaff_EBX);
                      _00___FRawStaticIndexBuffer__vfunc_0();
                      goto LAB_01580e8b;
                    }
                  }
                  piVar8 = *(int **)(*(int *)(unaff_EBP + 0xc) + 0x44);
                  *(undefined1 **)(unaff_EBP + 8) = &stack0xfffffff4;
                  piVar12 = piVar8;
                  Mercury__unknown_01576d50
                            ((int *)&stack0xfffffff4,unaff_EBP + 0xc,(int *)(unaff_EBP + 0xc));
                  *(undefined1 *)(unaff_EBP + -4) = 6;
                  *(undefined1 *)(unaff_EBP + -4) = 0;
                  UnAckedHandler_queueAckForPacket
                            (*(void **)(unaff_EBP + -0x18),(void *)(unaff_EBP + -0x68),piVar8,
                             piVar12);
                  *(undefined1 *)(unaff_EBP + -4) = 7;
                  if (*(int *)(unaff_EBP + -0x68) != 0) {
                    *(undefined1 *)(unaff_EBP + -4) = 0;
                    InterfaceElement__unknown_0158c100((int *)(unaff_EBP + -0x68));
                    goto LAB_01580f25;
                  }
                  if (*(char *)(unaff_EBP + -100) != '\0') {
                    *(undefined1 *)(unaff_EBP + -4) = 0;
                    InterfaceElement__unknown_0158c100((int *)(unaff_EBP + -0x68));
LAB_01580e8b:
                    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
                    Mercury__unknown_0157df90((int *)(unaff_EBP + 0xc));
                    ExceptionList = *(void **)(unaff_EBP + -0xc);
                    return 0;
                  }
                  Mercury__unknown_0157c020((int *)((int)this + 0xf8));
                  *(undefined1 *)(unaff_EBP + -4) = 0;
                  InterfaceElement__unknown_0158c100((int *)(unaff_EBP + -0x68));
                }
              }
              else {
                Mercury__unknown_015847f0(unaff_EBX);
                _00___FRawStaticIndexBuffer__vfunc_0();
                Mercury__unknown_0157c020((int *)((int)this + 0xf8));
              }
            }
          }
        }
      }
    }
  }
LAB_015810c0:
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  Mercury__unknown_0157df90((int *)(unaff_EBP + 0xc));
LAB_015810cf:
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return -4;
}


/* Mercury debug (Nub): Nub::processPendingEvents: Throwing REASON_GENERAL_NETWORK (1)- %s
    */

uint __thiscall Mercury_Nub_11(void *this,int *param_1)

{
  undefined4 *puVar1;
  char *pcVar2;
  undefined4 *puVar3;
  int iVar4;
  undefined1 *puVar5;
  int *piVar6;
  uint uVar7;
  bool bVar8;
  uint local_c8;
  undefined4 local_c4;
  int *local_c0;
  int *local_bc;
  undefined4 *local_b8;
  undefined4 *local_b4;
  undefined4 *puStack_b0;
  undefined1 *local_ac;
  void *local_a8;
  char *local_a4;
  int local_a0;
  int local_9c;
  int local_98;
  undefined **local_94;
  int local_90;
  uint local_8c;
  undefined4 local_88;
  undefined **ppuStack_84;
  undefined4 uStack_80;
  undefined4 uStack_7c;
  undefined2 uStack_78;
  undefined2 uStack_76;
  int local_74;
  int local_70;
  int local_6c;
  int iStack_68;
  int local_64;
  int local_60;
  int local_5c;
  int iStack_58;
  char *local_54;
  undefined **ppuStack_50;
  undefined4 uStack_4c;
  undefined4 uStack_48;
  undefined2 uStack_44;
  undefined2 uStack_42;
  undefined **local_40;
  undefined4 local_3c;
  uint local_38;
  undefined4 local_34;
  undefined **local_30 [3];
  undefined **local_24 [3];
  exception aeStack_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01793e01;
  local_c = ExceptionList;
  local_9c = *(int *)((int)this + 0xe0);
  ExceptionList = &local_c;
  local_a8 = this;
  do {
    local_c8 = 0;
    local_c4 = 0;
    pcVar2 = (char *)scalable_malloc();
    if (pcVar2 == (char *)0x0) {
      local_54 = "scalable_malloc failed";
      std::exception::exception((exception *)local_30,&local_54);
      local_30[0] = std::bad_alloc::vftable;
      local_4 = 0xffffffff;
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_30,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_4 = 1;
    local_a4 = pcVar2;
    puVar3 = FUN_0158a260((undefined4 *)pcVar2);
    if (puVar3 != (undefined4 *)0x0) {
      local_bc = puVar3 + 1;
      LOCK();
      local_64 = *local_bc;
      *local_bc = *local_bc + 1;
      UNLOCK();
    }
    local_4 = 2;
    local_b4 = puVar3;
    iVar4 = FUN_0158a200(puVar3,(SOCKET *)(*param_1 + 8),(char *)&local_c8);
    FUN_0158a3f0((int)puVar3);
    if (iVar4 < 1) {
      if (iVar4 == -1) {
        iVar4 = WSAGetLastError();
        if (iVar4 != 0x2733) {
          if (((iVar4 == 0x274d) || (iVar4 == 0x2751)) || (iVar4 == 0x2746)) {
            local_ac = (undefined1 *)FUN_00418e30(0xc);
            bVar8 = local_ac == (undefined1 *)0x0;
            if (bVar8) {
              puVar3 = (undefined4 *)0x0;
            }
            else {
              local_40 = Mercury::NubException::vftable;
              local_3c = 0xfffffffe;
              local_38 = local_c8;
              local_34 = local_c4;
              local_4 = CONCAT31((int3)((uint)local_4 >> 8),0xf);
              puVar3 = Mercury__unknown_0158d570(local_ac,(int)&local_40);
            }
            local_4 = 0x10;
            piVar6 = Mercury__unknown_0157df60(&local_60,(int)puVar3);
            local_4 = 0x11;
            tbb::internal::concurrent_queue_base_v3::internal_push
                      ((concurrent_queue_base_v3 *)((int)local_a8 + 0x138),piVar6);
            local_4 = CONCAT31(local_4._1_3_,0x10);
            Mercury__unknown_0157df90(&local_60);
            if (!bVar8) {
              local_40 = Mercury::NubException::vftable;
            }
          }
          else {
            strerror(iVar4);
            _00___FRawStaticIndexBuffer__vfunc_0();
            local_ac = (undefined1 *)FUN_00418e30(0xc);
            bVar8 = local_ac == (undefined1 *)0x0;
            if (bVar8) {
              puVar3 = (undefined4 *)0x0;
            }
            else {
              ppuStack_50 = Mercury::NubException::vftable;
              uStack_4c = 0xfffffffd;
              uStack_48 = 0;
              uStack_44 = 0;
              uStack_42 = 0;
              local_4 = CONCAT31((int3)((uint)local_4 >> 8),0x13);
              puVar3 = Mercury__unknown_0158d570(local_ac,(int)&ppuStack_50);
            }
            local_4 = 0x14;
            piVar6 = Mercury__unknown_0157df60(&iStack_58,(int)puVar3);
            local_4 = 0x15;
            tbb::internal::concurrent_queue_base_v3::internal_push
                      ((concurrent_queue_base_v3 *)((int)local_a8 + 0x138),piVar6);
            local_4 = CONCAT31(local_4._1_3_,0x14);
            Mercury__unknown_0157df90(&iStack_58);
            if (!bVar8) {
              ppuStack_50 = Mercury::NubException::vftable;
            }
          }
        }
      }
      else {
        piVar6 = _errno();
        strerror(*piVar6);
        pcVar2 = "Nub::processPendingEvents: Throwing REASON_GENERAL_NETWORK (1)- %s\n";
        _00___FRawStaticIndexBuffer__vfunc_0();
        puVar5 = (undefined1 *)scalable_malloc(0xc,pcVar2);
        if (puVar5 == (undefined1 *)0x0) {
          FUN_004099f0(aeStack_18);
                    /* WARNING: Subroutine does not return */
          _CxxThrowException(aeStack_18,(ThrowInfo *)&DAT_01d65cc8);
        }
        ppuStack_84 = Mercury::NubException::vftable;
        uStack_80 = 0xfffffffd;
        uStack_7c = 0;
        uStack_78 = 0;
        uStack_76 = 0;
        local_4 = CONCAT31((int3)((uint)local_4 >> 8),0xb);
        local_ac = puVar5;
        puStack_b0 = Mercury__unknown_0158d570(puVar5,(int)&ppuStack_84);
        if (puStack_b0 != (undefined4 *)0x0) {
          local_c0 = puStack_b0 + 1;
          LOCK();
          iStack_68 = *local_c0;
          *local_c0 = *local_c0 + 1;
          UNLOCK();
        }
        local_4 = 0xd;
        tbb::internal::concurrent_queue_base_v3::internal_push
                  ((concurrent_queue_base_v3 *)((int)this + 0x138),&puStack_b0);
        local_4 = CONCAT31(local_4._1_3_,0xc);
        Mercury__unknown_0157df90((int *)&puStack_b0);
        ppuStack_84 = Mercury::NubException::vftable;
      }
      local_4 = 0xffffffff;
      uVar7 = Mercury__unknown_0157df90((int *)&local_b4);
      ExceptionList = local_c;
      return uVar7 & 0xffffff00;
    }
    piVar6 = (int *)((int)this + 0xf0);
    LOCK();
    local_5c = *piVar6;
    *piVar6 = *piVar6 + 1;
    UNLOCK();
    piVar6 = (int *)((int)this + 0xe8);
    LOCK();
    local_70 = *piVar6;
    *piVar6 = *piVar6 + iVar4 + 0x1c;
    UNLOCK();
    puVar3[9] = iVar4;
    puVar1 = (undefined4 *)*param_1;
    local_bc = (int *)&stack0xffffff20;
    if (puVar1 != (undefined4 *)0x0) {
      puVar1[1] = puVar1[1] + 1;
    }
    local_ac = &stack0xffffff1c;
    piVar6 = puVar3 + 1;
    LOCK();
    local_6c = *piVar6;
    *piVar6 = *piVar6 + 1;
    UNLOCK();
    local_4._1_3_ = (undefined3)((uint)local_4 >> 8);
    local_4._0_1_ = 2;
    local_c0 = piVar6;
    iVar4 = Mercury__unknown_01581830(this,&local_c8,puVar3,puVar1);
    if (iVar4 != 0) {
      puVar5 = (undefined1 *)scalable_malloc();
      if (puVar5 == (undefined1 *)0x0) {
        local_a4 = "scalable_malloc failed";
        std::exception::exception((exception *)local_24,&local_a4);
        local_24[0] = std::bad_alloc::vftable;
        local_4 = CONCAT31(local_4._1_3_,2);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(local_24,(ThrowInfo *)&DAT_01d65cc8);
      }
      local_94 = Mercury::NubException::vftable;
      local_8c = local_c8;
      local_88 = local_c4;
      local_4 = CONCAT31(local_4._1_3_,7);
      local_ac = puVar5;
      local_90 = iVar4;
      local_b8 = Mercury__unknown_0158d570(puVar5,(int)&local_94);
      if (local_b8 != (undefined4 *)0x0) {
        local_c0 = local_b8 + 1;
        LOCK();
        local_74 = *local_c0;
        *local_c0 = *local_c0 + 1;
        UNLOCK();
      }
      local_4 = 9;
      tbb::internal::concurrent_queue_base_v3::internal_push
                ((concurrent_queue_base_v3 *)((int)this + 0x138),&local_b8);
      local_4 = CONCAT31(local_4._1_3_,8);
      if (local_b8 != (undefined4 *)0x0) {
        local_c0 = local_b8 + 1;
        LOCK();
        local_98 = *local_c0;
        *local_c0 = *local_c0 + -1;
        UNLOCK();
        if ((local_98 == 1 || local_98 + -1 < 0) && (local_b8 != (undefined4 *)0x0)) {
          (**(code **)*local_b8)();
        }
      }
      local_94 = Mercury::NubException::vftable;
    }
    local_4 = 0xffffffff;
    LOCK();
    local_a0 = *piVar6;
    *piVar6 = *piVar6 + -1;
    UNLOCK();
    local_c0 = piVar6;
    if (local_a0 == 1 || local_a0 + -1 < 0) {
      (**(code **)*puVar3)();
    }
    local_9c = local_9c + -1;
  } while (0 < local_9c);
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)local_9c >> 8),1);
}


/* WARNING: Removing unreachable block (ram,0x015822cc) */
/* WARNING: Removing unreachable block (ram,0x01582356) */
/* WARNING: Removing unreachable block (ram,0x01582530) */
/* WARNING: Removing unreachable block (ram,0x015827bc) */
/* WARNING: Removing unreachable block (ram,0x015827e8) */
/* WARNING: Removing unreachable block (ram,0x01582a41) */
/* WARNING: Removing unreachable block (ram,0x01582a6c) */
/* Mercury error in Nub: Nub::send( %s ): Failed for %s
   
   [String discovery] References: "Nub::sendInternal" */

int Mercury_Nub_4(uint *param_1,int *param_2,void *param_3)

{
  byte *pbVar1;
  undefined1 *puVar2;
  byte bVar3;
  undefined1 uVar4;
  undefined2 uVar5;
  int *piVar6;
  bool bVar7;
  int iVar8;
  void *this;
  byte bVar9;
  int iVar10;
  int iVar11;
  undefined4 uVar12;
  int *piVar13;
  int *piVar14;
  char *pcVar15;
  undefined4 *puVar16;
  undefined4 *puVar17;
  int *piStack_e4;
  undefined4 *puStack_e0;
  undefined4 *puStack_d8;
  undefined4 *puStack_d4;
  int *piStack_d0;
  undefined4 *puStack_cc;
  char *pcStack_c8;
  undefined4 *puStack_c4;
  uint uStack_c0;
  uint uStack_bc;
  uint *puStack_b8;
  uint local_b4;
  byte *pbStack_b0;
  int *piStack_a8;
  void *pvStack_a4;
  void *local_a0;
  int iStack_9c;
  int iStack_98;
  undefined4 *puStack_90;
  int iStack_8c;
  ushort *puStack_88;
  int iStack_84;
  int iStack_7c;
  int iStack_78;
  int iStack_74;
  int iStack_70;
  int iStack_6c;
  int iStack_68;
  int iStack_60;
  int iStack_5c;
  int iStack_58;
  int iStack_54;
  int iStack_50;
  int iStack_4c;
  char *apcStack_48 [2];
  int *piStack_40;
  int iStack_3c;
  int iStack_38;
  char *pcStack_34;
  undefined **appuStack_30 [3];
  undefined **appuStack_24 [3];
  undefined **appuStack_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01793f0b;
  pvStack_c = ExceptionList;
  local_b4 = 0;
  ExceptionList = &pvStack_c;
  if ((*(char *)((int)param_2 + 0x35) != '\0') &&
     (ExceptionList = &pvStack_c, param_3 == (void *)0x0)) {
    ExceptionList = &pvStack_c;
    _00___FRawStaticIndexBuffer__vfunc_0();
  }
  bVar7 = false;
  Mercury_Bundle_2(param_2);
  bVar3 = *(byte *)(param_2[1] + 0x54);
  if (bVar3 != 0) {
    iVar10 = Mercury__unknown_01578e60((int)param_2);
    piStack_40 = (int *)param_2[0x13];
    if ((int *)param_2[0x14] < piStack_40) {
      _invalid_parameter_noinfo();
    }
    pvStack_a4 = (void *)0x0;
    if (((bVar3 & 0x10) != 0) && (param_3 != (void *)0x0)) {
      pvStack_a4 = param_3;
    }
    puStack_e0 = (undefined4 *)param_2[1];
    iStack_8c = 0;
    iStack_98 = 0;
    if (puStack_e0 != (undefined4 *)0x0) {
      puStack_b8 = puStack_e0 + 1;
      LOCK();
      piStack_d0 = (int *)*puStack_b8;
      *puStack_b8 = *puStack_b8 + 1;
      UNLOCK();
    }
    uStack_4 = 0;
    puVar16 = puStack_e0;
    while (puVar16 != (undefined4 *)0x0) {
      puVar16[9] = puVar16[9] + puVar16[10];
      bVar3 = *(byte *)(puVar16 + 0x15);
      piVar13 = puVar16 + 9;
      pbVar1 = (byte *)(puVar16 + 0x15);
      bVar9 = bVar3 & 2;
      pbStack_b0 = pbVar1;
      if (bVar9 == 0) {
LAB_015824a0:
        if (bVar9 == 0) {
LAB_015824cb:
          if ((param_2[0xf] != 0) && (param_2[0x10] - param_2[0xf] >> 2 != 0)) {
            _00___FRawStaticIndexBuffer__vfunc_0();
          }
        }
        else if ((param_2[0xf] == 0) || (param_2[0x10] - param_2[0xf] >> 2 == 0)) {
          _00___FRawStaticIndexBuffer__vfunc_0();
          *pbVar1 = *pbVar1 & 0xfd;
        }
        else if (bVar9 == 0) goto LAB_015824cb;
      }
      else {
        piStack_d0 = param_2 + 0xe;
        if ((param_2[0xf] == 0) || (param_2[0x10] - param_2[0xf] >> 2 == 0)) goto LAB_015824a0;
        puVar17 = (undefined4 *)param_2[0xf];
        iVar11 = *piVar13;
        puStack_88 = (ushort *)0x0;
        if ((undefined4 *)param_2[0x10] < puVar17) {
          _invalid_parameter_noinfo();
        }
        while( true ) {
          puStack_90 = puVar17;
          puVar17 = (undefined4 *)param_2[0x10];
          if (puVar17 < (undefined4 *)param_2[0xf]) {
            _invalid_parameter_noinfo();
          }
          if (puStack_90 == puVar17) break;
          if ((undefined4 *)param_2[0x10] <= puStack_90) {
            _invalid_parameter_noinfo();
          }
          piVar14 = (int *)*puStack_90;
          iVar8 = piVar14[3];
          *piVar13 = *piVar13 + -2;
          *(short *)(*piVar13 + 0x54 + (int)puVar16) = (short)iVar8;
          puStack_88 = (ushort *)(*piVar13 + 0x54 + (int)puVar16);
          *piVar13 = *piVar13 - (int)(short)piVar14[3];
          puVar2 = (undefined1 *)(*piVar13 + 0x54 + (int)puVar16);
          *puVar2 = (char)piVar14[1];
          piStack_e4 = (int *)(puVar2 + 1);
          piStack_d0 = (int *)piVar14[5];
          piVar6 = piStack_d0;
          if ((int *)piVar14[6] < piStack_d0) {
            _invalid_parameter_noinfo();
            piVar6 = piStack_d0;
          }
          while( true ) {
            piStack_a8 = piVar6;
            piStack_d0 = (int *)piVar14[6];
            if (piStack_d0 < (int *)piVar14[5]) {
              _invalid_parameter_noinfo();
            }
            if (piStack_a8 == piStack_d0) break;
            if ((int *)piVar14[6] <= piStack_a8) {
              _invalid_parameter_noinfo();
            }
            if ((int *)piVar14[6] <= piStack_a8) {
              _invalid_parameter_noinfo();
            }
            memcpy(piStack_e4,(void *)*piStack_a8,(uint)*(ushort *)(piStack_a8 + 1));
            if ((int *)piVar14[6] <= piStack_a8) {
              _invalid_parameter_noinfo();
            }
            piStack_e4 = (int *)((int)piStack_e4 + (uint)*(ushort *)(piStack_a8 + 1));
            if ((int *)piVar14[6] <= piStack_a8) {
              _invalid_parameter_noinfo();
            }
            piVar6 = piStack_a8 + 2;
          }
          *piStack_e4 = piVar14[2];
          if ((*(byte *)(piVar14 + 1) & 2) != 0) {
            memcpy(piStack_e4 + 1,*(void **)(*piVar14 + 0x3c),(uint)*(ushort *)(*piVar14 + 0x40));
          }
          Mercury__unknown_015847f0(param_1);
          _00___FRawStaticIndexBuffer__vfunc_0();
          puVar17 = (undefined4 *)*piVar14;
          if (puVar17 != (undefined4 *)0x0) {
            piVar6 = puVar17 + 1;
            LOCK();
            iStack_84 = *piVar6;
            *piVar6 = *piVar6 + -1;
            UNLOCK();
            puVar16 = puStack_e0;
            if ((iStack_84 == 1 || iStack_84 + -1 < 0) && (puVar17 != (undefined4 *)0x0)) {
              (**(code **)*puVar17)();
            }
          }
          *piVar14 = 0;
          if ((undefined4 *)param_2[0x10] <= puStack_90) {
            _invalid_parameter_noinfo();
          }
          puVar17 = puStack_90 + 1;
        }
        *puStack_88 = ~*puStack_88;
        puVar16[0xf] = *piVar13 + 0x54 + (int)puVar16;
        *(short *)(puVar16 + 0x10) = (short)iVar11 - (short)*piVar13;
      }
      piVar14 = piStack_40;
      if ((bVar3 & 4) != 0) {
        uVar4 = *(undefined1 *)(puVar16 + 0xe);
        *piVar13 = *piVar13 + -1;
        *(undefined1 *)(*piVar13 + 0x54 + (int)puVar16) = uVar4;
        while( true ) {
          piVar6 = (int *)param_2[0x14];
          if (piVar6 < (int *)param_2[0x13]) {
            _invalid_parameter_noinfo();
          }
          if (piVar14 == piVar6) break;
          if ((int *)param_2[0x14] <= piVar14) {
            _invalid_parameter_noinfo();
          }
          if ((undefined4 *)*piVar14 != puVar16) break;
          if ((int *)param_2[0x14] <= piVar14) {
            _invalid_parameter_noinfo();
          }
          iVar11 = piVar14[1];
          *piVar13 = *piVar13 + -4;
          *(int *)(*piVar13 + 0x54 + (int)puVar16) = iVar11;
          if ((int *)param_2[0x14] <= piVar14) {
            _invalid_parameter_noinfo();
          }
          piVar14 = piVar14 + 2;
        }
      }
      piStack_40 = piVar14;
      this = pvStack_a4;
      if ((bVar3 & 0x40) != 0) {
        if (pvStack_a4 == (void *)0x0) {
          iVar11 = *(int *)((int)local_a0 + 0x98);
          *(uint *)((int)local_a0 + 0x98) = iVar11 + 1U & 0xfffffff;
        }
        else {
          iVar11 = FUN_0158bb40((int)pvStack_a4);
        }
        puVar16[0x11] = iVar11;
        *piVar13 = *piVar13 + -4;
        *(int *)(*piVar13 + 0x54 + (int)puVar16) = iVar11;
        if (puVar16 == (undefined4 *)param_2[1]) {
          iStack_8c = puVar16[0x11];
          iStack_98 = iStack_8c + -1 + iVar10;
        }
      }
      if ((bVar3 & 1) != 0) {
        uVar5 = *(undefined2 *)(puVar16 + 0xc);
        *piVar13 = *piVar13 + -2;
        *(undefined2 *)(*piVar13 + 0x54 + (int)puVar16) = uVar5;
      }
      if ((bVar3 & 0x20) != 0) {
        *piVar13 = *piVar13 + -4;
        *(int *)(*piVar13 + 0x54 + (int)puVar16) = iStack_98;
        *piVar13 = *piVar13 + -4;
        *(int *)(*piVar13 + 0x54 + (int)puVar16) = iStack_8c;
      }
      if (param_3 == (void *)0x0) {
        *pbStack_b0 = *pbStack_b0 & 0xf7;
      }
      else {
        *pbStack_b0 = *pbStack_b0 | 8;
      }
      if ((bVar3 & 0x10) != 0) {
        if (this == (void *)0x0) {
          _00___FRawStaticIndexBuffer__vfunc_0();
        }
        else {
          pbStack_b0 = &stack0xfffffef4;
          piVar13 = puVar16 + 1;
          LOCK();
          iStack_4c = *piVar13;
          *piVar13 = *piVar13 + 1;
          UNLOCK();
          uStack_4 = uStack_4 & 0xffffff00;
          FUN_01579180(param_2,puVar16,&uStack_bc,&uStack_c0);
          pbStack_b0 = &stack0xfffffefc;
          LOCK();
          iStack_38 = *piVar13;
          *piVar13 = *piVar13 + 1;
          UNLOCK();
          uStack_4 = uStack_4 & 0xffffff00;
          uVar12 = Mercury__unknown_0158c2a0(this,puStack_e0[0x11],puStack_e0);
          bVar7 = (bool)(bVar7 | (char)uVar12 == '\0');
          puVar16 = puStack_e0;
        }
      }
      piVar13 = Mercury__unknown_0158a850(puVar16,(int *)&puStack_cc);
      uStack_4._0_1_ = 1;
      if (puVar16 != (undefined4 *)*piVar13) {
        piVar14 = puVar16 + 1;
        LOCK();
        iStack_7c = *piVar14;
        *piVar14 = *piVar14 + -1;
        UNLOCK();
        if (iStack_7c == 1 || iStack_7c + -1 < 0) {
          (**(code **)*puStack_e0)();
        }
        puVar16 = (undefined4 *)*piVar13;
        puStack_e0 = puVar16;
        if (puVar16 != (undefined4 *)0x0) {
          piVar13 = puVar16 + 1;
          LOCK();
          iStack_54 = *piVar13;
          *piVar13 = *piVar13 + 1;
          UNLOCK();
        }
      }
      uStack_4 = (uint)uStack_4._1_3_ << 8;
      if (puStack_cc != (undefined4 *)0x0) {
        piVar13 = puStack_cc + 1;
        LOCK();
        iStack_6c = *piVar13;
        *piVar13 = *piVar13 + -1;
        UNLOCK();
        puVar16 = puStack_e0;
        if ((iStack_6c == 1 || iStack_6c + -1 < 0) && (puStack_cc != (undefined4 *)0x0)) {
          (**(code **)*puStack_cc)();
        }
      }
    }
    uStack_4 = 0xffffffff;
  }
  if (param_3 == (void *)0x0) {
    iStack_74 = 0;
    piVar13 = &iStack_74;
    uStack_4 = 5;
    local_b4 = 1;
  }
  else {
    piVar13 = (int *)Mercury__unknown_0158b970((int)param_3);
  }
  piVar13 = (int *)*piVar13;
  puStack_b8 = (uint *)piVar13;
  if (piVar13 != (int *)0x0) {
    FUN_00457e40((int)piVar13);
  }
  uStack_4._0_1_ = 8;
  uStack_4._1_3_ = 0;
  if ((local_b4 & 1) != 0) {
    local_b4 = local_b4 & 0xfffffffe;
    FUN_004585a0(&iStack_74);
  }
  puVar16 = (undefined4 *)param_2[1];
  puStack_d8 = puVar16;
  if (puVar16 != (undefined4 *)0x0) {
    piVar14 = puVar16 + 1;
    LOCK();
    iStack_60 = *piVar14;
    *piVar14 = *piVar14 + 1;
    UNLOCK();
  }
  while (puVar16 != (undefined4 *)0x0) {
    puVar17 = puVar16;
    if (piVar13 == (int *)0x0) {
      if (puVar16 != (undefined4 *)0x0) {
        piVar14 = puVar16 + 1;
        LOCK();
        iStack_3c = *piVar14;
        *piVar14 = *piVar14 + 1;
        UNLOCK();
        puVar17 = puStack_d8;
      }
      uStack_4 = CONCAT31(uStack_4._1_3_,9);
      iVar10 = Mercury__unknown_015814c0(local_a0,param_1,puVar16);
    }
    else {
      if (puVar16 != (undefined4 *)0x0) {
        piVar14 = puVar16 + 1;
        LOCK();
        iStack_58 = *piVar14;
        *piVar14 = *piVar14 + 1;
        UNLOCK();
        puVar17 = puStack_d8;
      }
      uStack_4 = CONCAT31(uStack_4._1_3_,9);
      iVar10 = (**(code **)(*piVar13 + 4))();
    }
    if (iVar10 != 0) {
      Mercury__unknown_00de15e0(iVar10);
      Mercury__unknown_015847f0(param_1);
      _00___FRawStaticIndexBuffer__vfunc_0();
      *(int *)((int)local_a0 + 0x114) = *(int *)((int)local_a0 + 0x114) + 1;
      uStack_4 = CONCAT31(uStack_4._1_3_,8);
      if (puVar17 != (undefined4 *)0x0) {
        piVar14 = puVar17 + 1;
        LOCK();
        iStack_68 = *piVar14;
        *piVar14 = *piVar14 + -1;
        UNLOCK();
        if (iStack_68 == 1 || iStack_68 + -1 < 0) {
          (**(code **)*puStack_d8)();
        }
      }
      uStack_4 = 0xffffffff;
      if (piVar13 == (int *)0x0) {
        ExceptionList = pvStack_c;
        return iVar10;
      }
      iVar11 = FUN_00457e50((int)piVar13);
      if (iVar11 != 1) {
        ExceptionList = pvStack_c;
        return iVar10;
      }
      (**(code **)*piVar13)();
      ExceptionList = pvStack_c;
      return iVar10;
    }
    piVar14 = Mercury__unknown_0158a850(puVar17,(int *)&puStack_d4);
    uStack_4._0_1_ = 10;
    if (puVar17 != (undefined4 *)*piVar14) {
      if (puVar17 != (undefined4 *)0x0) {
        piVar6 = puVar17 + 1;
        LOCK();
        iStack_78 = *piVar6;
        *piVar6 = *piVar6 + -1;
        UNLOCK();
        if (iStack_78 == 1 || iStack_78 + -1 < 0) {
          (**(code **)*puStack_d8)();
        }
      }
      puVar17 = (undefined4 *)*piVar14;
      puStack_d8 = puVar17;
      if (puVar17 != (undefined4 *)0x0) {
        piVar14 = puVar17 + 1;
        LOCK();
        iStack_50 = *piVar14;
        *piVar14 = *piVar14 + 1;
        UNLOCK();
      }
    }
    uStack_4._0_1_ = 9;
    puVar16 = puVar17;
    if (puStack_d4 != (undefined4 *)0x0) {
      piVar14 = puStack_d4 + 1;
      LOCK();
      iStack_9c = *piVar14;
      *piVar14 = *piVar14 + -1;
      UNLOCK();
      puVar16 = puStack_d8;
      if ((iStack_9c == 1 || iStack_9c + -1 < 0) && (puStack_d4 != (undefined4 *)0x0)) {
        (**(code **)*puStack_d4)();
      }
    }
  }
  uStack_4._0_1_ = 8;
  *(int *)((int)local_a0 + 0xfc) = *(int *)((int)local_a0 + 0xfc) + 1;
  *(int *)((int)local_a0 + 0x104) = *(int *)((int)local_a0 + 0x104) + param_2[0x1b];
  if (param_2[9] == 0) {
    iVar10 = 0;
  }
  else {
    iVar10 = param_2[10] - param_2[9] >> 3;
  }
  *(int *)((int)local_a0 + 0x108) = *(int *)((int)local_a0 + 0x108) + iVar10;
  if (bVar7) {
    pcVar15 = (char *)scalable_malloc();
    pcStack_c8 = pcVar15;
    if (param_3 == (void *)0x0) {
      if (pcVar15 == (char *)0x0) {
        apcStack_48[0] = "scalable_malloc failed";
        std::exception::exception((exception *)appuStack_24,apcStack_48);
        appuStack_24[0] = std::bad_alloc::vftable;
        uStack_4 = CONCAT31(uStack_4._1_3_,8);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(appuStack_24,(ThrowInfo *)&DAT_01d65cc8);
      }
      *(undefined ***)pcVar15 = Mercury::NubException::vftable;
      pcVar15[4] = -6;
      pcVar15[5] = -1;
      pcVar15[6] = -1;
      pcVar15[7] = -1;
      pcVar15[8] = '\0';
      pcVar15[9] = '\0';
      pcVar15[10] = '\0';
      pcVar15[0xb] = '\0';
      pcVar15[0xc] = '\0';
      pcVar15[0xd] = '\0';
      pcVar15[0xe] = '\0';
      pcVar15[0xf] = '\0';
    }
    else {
      if (pcVar15 == (char *)0x0) {
        pcStack_34 = "scalable_malloc failed";
        std::exception::exception((exception *)appuStack_30,&pcStack_34);
        appuStack_30[0] = std::bad_alloc::vftable;
        uStack_4 = CONCAT31(uStack_4._1_3_,8);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(appuStack_30,(ThrowInfo *)&DAT_01d65cc8);
      }
      uStack_4._0_1_ = 0xf;
      puVar16 = (undefined4 *)Mercury__unknown_0158b960((int)param_3);
      *(undefined ***)pcVar15 = Mercury::NubException::vftable;
      pcVar15[4] = -6;
      pcVar15[5] = -1;
      pcVar15[6] = -1;
      pcVar15[7] = -1;
      *(undefined4 *)(pcVar15 + 8) = *puVar16;
      *(undefined4 *)(pcVar15 + 0xc) = puVar16[1];
    }
    uStack_4._0_1_ = 8;
    pcStack_c8 = (char *)scalable_malloc();
    if (pcStack_c8 == (char *)0x0) {
      pcStack_c8 = "scalable_malloc failed";
      std::exception::exception((exception *)appuStack_18,&pcStack_c8);
      appuStack_18[0] = std::bad_alloc::vftable;
      uStack_4 = CONCAT31(uStack_4._1_3_,8);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(appuStack_18,(ThrowInfo *)&DAT_01d65cc8);
    }
    uStack_4._0_1_ = 0x13;
    puStack_c4 = Mercury__unknown_0158d570(pcStack_c8,(int)pcVar15);
    if (puStack_c4 != (undefined4 *)0x0) {
      piVar14 = puStack_c4 + 1;
      LOCK();
      iStack_5c = *piVar14;
      *piVar14 = *piVar14 + 1;
      UNLOCK();
    }
    uStack_4._0_1_ = 0x14;
    tbb::internal::concurrent_queue_base_v3::internal_push
              ((concurrent_queue_base_v3 *)((int)local_a0 + 0x138),&puStack_c4);
    uStack_4 = CONCAT31(uStack_4._1_3_,8);
    if (puStack_c4 != (undefined4 *)0x0) {
      piVar14 = puStack_c4 + 1;
      LOCK();
      iStack_70 = *piVar14;
      *piVar14 = *piVar14 + -1;
      UNLOCK();
      if ((iStack_70 + -1 < 1) && (puStack_c4 != (undefined4 *)0x0)) {
        (**(code **)*puStack_c4)();
      }
    }
  }
  uStack_4 = 0xffffffff;
  if ((piVar13 != (int *)0x0) && (iVar10 = FUN_00457e50((int)piVar13), iVar10 == 1)) {
    (**(code **)*piVar13)();
  }
  ExceptionList = pvStack_c;
  return 0;
}


/* Mercury debug string: Mercury::Nub::addListeningSocket */

uint __thiscall Mercury_Nub_addListeningSocket(void *this,undefined1 *param_1)

{
  SOCKET *pSVar1;
  undefined4 *puVar2;
  SOCKET SVar3;
  int iVar4;
  uint uVar5;
  undefined1 *puVar6;
  undefined4 uVar7;
  char acStack_3d [5];
  undefined4 *local_38;
  undefined4 *local_34;
  uint local_30;
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017940cc;
  pvStack_c = ExceptionList;
  acStack_3d[1] = '\0';
  acStack_3d[2] = '\0';
  acStack_3d[3] = '\0';
  acStack_3d[4] = '\0';
  ExceptionList = &pvStack_c;
  local_34 = (undefined4 *)scalable_malloc(0x24);
  if (local_34 == (undefined4 *)0x0) {
    FUN_004099f0((exception *)&local_34);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(&local_34,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  puVar2 = FUN_01583070(local_34);
  if (puVar2 != (undefined4 *)0x0) {
    puVar2[1] = puVar2[1] + 1;
  }
  local_4 = 1;
  local_38 = puVar2;
  FUN_00e68470((void *)((int)this + 0x78),(int *)&local_38);
  pSVar1 = puVar2 + 2;
  SVar3 = socket(2,2,0);
  *pSVar1 = SVar3;
  if (SVar3 == 0xffffffff) {
    iVar4 = WSAGetLastError();
    if (iVar4 == 0x276d) {
      FUN_015848f0();
      SVar3 = socket(2,2,0);
      *pSVar1 = SVar3;
    }
  }
  if (*pSVar1 == 0xffffffff) {
    local_34 = (undefined4 *)0x0;
    local_30 = 6;
    Mercury__unknown_00a351d0((int *)&local_34,"%s: couldn\'t create listening socket\n");
  }
  iVar4 = *(int *)(param_1 + 0x14);
  puVar6 = param_1;
  if (iVar4 == 0) {
    local_10 = 0xf;
    param_1 = (undefined1 *)((uint)param_1 & 0xffffff00);
    local_14 = 0;
    std::char_traits<char>::assign((char *)local_24,(char *)&param_1);
    uVar5 = std::char_traits<char>::length("");
    FUN_004377d0(auStack_28,"",uVar5);
    puVar6 = auStack_28;
    local_4 = CONCAT31(local_4._1_3_,2);
    acStack_3d[1] = '\x01';
    acStack_3d[2] = '\0';
    acStack_3d[3] = '\0';
    acStack_3d[4] = '\0';
  }
  uVar7 = Mercury_Nub_9(this,(u_long)puVar6,pSVar1);
  param_1 = (undefined1 *)CONCAT31(param_1._1_3_,(char)uVar7);
  local_4 = 1;
  if (iVar4 == 0) {
    acStack_3d[1] = '\0';
    acStack_3d[2] = '\0';
    acStack_3d[3] = '\0';
    acStack_3d[4] = '\0';
    if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(local_24[0]);
    }
    local_10 = 0xf;
    acStack_3d[0] = '\0';
    local_14 = 0;
    std::char_traits<char>::assign((char *)local_24,acStack_3d);
  }
  if ((char)param_1 != '\0') {
    iVar4 = FUN_01577070((int)this);
    uVar5 = *(uint *)(iVar4 + 8);
    if (uVar5 < *(uint *)(iVar4 + 4)) {
      _invalid_parameter_noinfo();
    }
    local_30 = uVar5;
    if ((*(uint *)(iVar4 + 8) < uVar5 - 8) || (uVar5 - 8 < *(uint *)(iVar4 + 4))) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)(iVar4 + 8) <= (undefined4 *)(uVar5 - 8)) {
      _invalid_parameter_noinfo();
    }
    puVar2[3] = *(undefined4 *)(uVar5 - 8);
    puVar2[4] = *(undefined4 *)(uVar5 - 4);
    Mercury__unknown_015847f0(puVar2 + 3);
    _00___FRawStaticIndexBuffer__vfunc_0();
  }
  uVar7 = 0xffffffff;
  local_4 = 0xffffffff;
  puVar2[1] = puVar2[1] + -1;
  if ((int)puVar2[1] < 1) {
    uVar7 = (**(code **)*puVar2)(1);
  }
  ExceptionList = pvStack_c;
  return CONCAT31((int3)((uint)uVar7 >> 8),(char)param_1);
}


/* Mercury debug (Nub): NubPrime::closeConnectionInternal: Calling handle exception for outstanding
   repl */

void __thiscall Mercury_Nub_15(void *this,int *param_1,int param_2)

{
  int *piVar1;
  int iVar2;
  void *pvVar3;
  void *this_00;
  void *local_24;
  int *piStack_20;
  int iStack_1c;
  undefined4 local_18;
  undefined2 local_14;
  undefined2 local_12;
  
  local_18 = 0;
  local_14 = 0;
  local_12 = 0;
  local_24 = this;
  (**(code **)(*param_1 + 4))(&local_18);
  piStack_20 = (int *)**(int **)((int)this + 0xa0);
  this_00 = (void *)((int)this + 0x9c);
  local_24 = this_00;
  while( true ) {
    pvVar3 = local_24;
    piVar1 = *(int **)((int)this + 0xa0);
    if ((local_24 == (void *)0x0) || (local_24 != this_00)) {
      _invalid_parameter_noinfo();
    }
    if (piStack_20 == piVar1) break;
    if (pvVar3 == (void *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (piStack_20 == *(int **)((int)pvVar3 + 4)) {
      _invalid_parameter_noinfo();
    }
    iVar2 = piStack_20[4];
    if ((*(int *)(iVar2 + 0x20) == iStack_1c) && (*(short *)(iVar2 + 0x24) == (short)local_18)) {
      Mercury__unknown_015847f0(&iStack_1c);
      _00___FRawStaticIndexBuffer__vfunc_0();
      if (*(int *)(iVar2 + 8) != 0) {
        Mercury__unknown_01578670(*(int *)(iVar2 + 8));
      }
      (**(code **)(**(int **)(iVar2 + 0xc) + 8))(param_1,*(undefined4 *)(iVar2 + 0x10));
      piVar1 = piStack_20;
      FUN_00e096b0((int *)&local_24);
      FUN_00c6a1e0(this_00,&local_14,pvVar3,piVar1);
    }
    else {
      FUN_00e096b0((int *)&local_24);
    }
  }
  if (param_2 != 0) {
    Mercury__unknown_00de15e0(param_1[1]);
    Mercury__unknown_015836d0();
    return;
  }
  Mercury__unknown_00de15e0(param_1[1]);
  Mercury__unknown_015836d0();
  return;
}


/* Mercury debug string: Mercury::Nub::writeConnection */

uint __thiscall Mercury_Nub_writeConnection(void *this,int param_1)

{
  int *piVar1;
  uint in_EAX;
  int iVar2;
  int iVar3;
  undefined4 uVar4;
  int local_18 [2];
  sockaddr local_10;
  
  if (*(int *)(param_1 + 0x1c) != 0) {
    do {
      piVar1 = (int *)**(int **)(param_1 + 0x18);
      if (piVar1 == *(int **)(param_1 + 0x18)) {
        _invalid_parameter_noinfo();
      }
      local_10.sa_data._2_4_ = piVar1[2];
      iVar2 = piVar1[4];
      local_10.sa_data._0_2_ = *(undefined2 *)(piVar1 + 3);
      local_10.sa_family = 2;
      iVar2 = sendto(*(SOCKET *)(param_1 + 8),(char *)(iVar2 + 0x54),
                     *(int *)(iVar2 + 0x28) + *(int *)(iVar2 + 0x24),0,&local_10,0x10);
      if (iVar2 != *(int *)(piVar1[4] + 0x28) + *(int *)(piVar1[4] + 0x24)) {
        *(int *)((int)this + 0x110) = *(int *)((int)this + 0x110) + 1;
        iVar3 = WSAGetLastError();
        strerror(iVar3);
        if (iVar2 == -1) {
          Mercury__unknown_015847f0(piVar1 + 2);
          uVar4 = Mercury__unknown_015836d0();
          return CONCAT31((int3)((uint)uVar4 >> 8),1);
        }
        Mercury__unknown_015847f0(piVar1 + 2);
        uVar4 = _00___FRawStaticIndexBuffer__vfunc_0();
        return CONCAT31((int3)((uint)uVar4 >> 8),1);
      }
      LOCK();
      *(int *)((int)this + 0xe4) = *(int *)((int)this + 0xe4) + iVar2 + 0x1c;
      UNLOCK();
      LOCK();
      *(int *)((int)this + 0xec) = *(int *)((int)this + 0xec) + 1;
      UNLOCK();
      in_EAX = FUN_01581150((void *)(param_1 + 0x14),local_18,param_1 + 0x14,
                            (int *)**(undefined4 **)(param_1 + 0x18));
    } while (*(int *)(param_1 + 0x1c) != 0);
  }
  return in_EAX & 0xffffff00;
}


/* Mercury debug string: Mercury::Nub::Nub */

undefined4 * __thiscall
Mercury_Nub_Nub(void *this,ushort param_1,undefined2 param_2,void *param_3,undefined4 *param_4,
               int param_5,undefined4 param_6,undefined4 param_7)

{
  ulonglong uVar1;
  undefined4 *puVar2;
  void *this_00;
  uint uVar3;
  longlong *plVar4;
  int iVar5;
  undefined4 *puVar6;
  int iStack_50;
  undefined4 local_4c;
  undefined2 auStack_48 [4];
  exception local_40 [12];
  exception local_34 [12];
  undefined1 auStack_28 [4];
  undefined4 local_24 [4];
  undefined4 local_14;
  uint local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017942c0;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_01577960(this,param_1,param_2,0,0,(short)param_3,(short)param_4);
  puVar6 = (undefined4 *)((int)this + 0x70);
  *puVar6 = Mercury::InputMessageHandler::vftable;
  *(undefined ***)((int)this + 0x74) = Mercury::TimerExpiryHandler::vftable;
  local_4._1_3_ = 0;
  *(undefined ***)this = Mercury::Nub::vftable;
  *puVar6 = Mercury::Nub::vftable;
  *(undefined ***)((int)this + 0x74) = Mercury::Nub::vftable;
  *(undefined4 *)((int)this + 0x7c) = 0;
  *(undefined4 *)((int)this + 0x80) = 0;
  *(undefined4 *)((int)this + 0x84) = 0;
  local_4._0_1_ = 3;
  FUN_01580620((int)this + 0x88);
  local_4._0_1_ = 4;
  uVar1 = rdtsc();
  local_4c = (undefined4)(uVar1 >> 0x20);
  iVar5 = (int)((uVar1 & 0xffffffff) * 0x4f8b588f >> 0x20);
  *(uint *)((int)this + 0x94) =
       (int)uVar1 + (((uint)((int)uVar1 - iVar5) >> 1) + iVar5 >> 0x10) * -100000 + 0x2775;
  *(undefined4 *)((int)this + 0x98) = 1;
  FUN_0157ce00((int)this + 0x9c);
  *(undefined4 *)((int)this + 0xa8) = 0;
  local_4._0_1_ = 6;
  FUN_015839a0((void *)((int)this + 0xac),0x100);
  local_4._0_1_ = 7;
  *(undefined1 *)((int)this + 0xc2) = 0;
  *(undefined4 *)((int)this + 0xc4) = 0;
  FUN_01120e80((int)this + 200);
  local_4._0_1_ = 8;
  FUN_01120e80((int)this + 0xd4);
  local_4._0_1_ = 9;
  *(undefined4 *)((int)this + 0xe0) = param_7;
  *(undefined4 *)((int)this + 0xfc) = 0;
  *(undefined4 *)((int)this + 0x100) = 0;
  *(undefined4 *)((int)this + 0x104) = 0;
  *(undefined4 *)((int)this + 0x108) = 0;
  *(undefined4 *)((int)this + 0x10c) = 0;
  *(undefined4 *)((int)this + 0x110) = 0;
  *(undefined4 *)((int)this + 0x114) = 0;
  *(undefined4 *)((int)this + 0x118) = 0;
  *(undefined4 *)((int)this + 0x11c) = 0;
  puVar2 = (undefined4 *)scalable_malloc(4);
  if (puVar2 == (undefined4 *)0x0) {
    FUN_004099f0(local_40);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_40,(ThrowInfo *)&DAT_01d65cc8);
  }
  *puVar2 = this;
  *(undefined4 **)((int)this + 0x120) = puVar2;
  local_4._0_1_ = 0xb;
  param_4 = puVar2;
  this_00 = (void *)scalable_malloc(0x34);
  if (this_00 == (void *)0x0) {
    FUN_004099f0(local_34);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_34,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4._0_1_ = 0xc;
  local_10 = 0xf;
  param_4 = (undefined4 *)((uint)param_4 & 0xffffff00);
  local_14 = 0;
  param_3 = this_00;
  std::char_traits<char>::assign((char *)local_24,(char *)&param_4);
  uVar3 = std::char_traits<char>::length("NetworkThread for ExternalNub");
  FUN_004377d0(auStack_28,"NetworkThread for ExternalNub",uVar3);
  local_4 = CONCAT31(local_4._1_3_,0xd);
  puVar2 = FUN_0157aea0(this_00,*(undefined4 *)((int)this + 0x120),auStack_28);
  *(undefined4 **)((int)this + 0x124) = puVar2;
  local_4 = 0x10;
  if (0xf < local_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_24[0]);
  }
  local_10 = 0xf;
  param_3 = (void *)((uint)param_3 & 0xffffff00);
  local_14 = 0;
  std::char_traits<char>::assign((char *)local_24,(char *)&param_3);
  *(undefined4 *)((int)this + 0x128) = 0;
  FUN_015810f0((int)this + 300);
  local_4._0_1_ = 0x11;
  tbb::internal::concurrent_queue_base_v3::concurrent_queue_base_v3
            ((concurrent_queue_base_v3 *)((int)this + 0x138),4);
  *(undefined ***)((int)this + 0x138) =
       tbb::
       concurrent_queue<class_CME::RefCountedObj<class_Mercury::ClientMessage>,class_tbb::cache_aligned_allocator<class_CME::RefCountedObj<class_Mercury::ClientMessage>_>_>
       ::vftable;
  local_4._0_1_ = 0x13;
  tbb::internal::concurrent_queue_base_v3::concurrent_queue_base_v3
            ((concurrent_queue_base_v3 *)((int)this + 0x150),4);
  *(undefined ***)((int)this + 0x150) =
       tbb::
       concurrent_queue<class_CME::RefCountedObj<class_Mercury::ClientMessage>,class_tbb::cache_aligned_allocator<class_CME::RefCountedObj<class_Mercury::ClientMessage>_>_>
       ::vftable;
  local_4._0_1_ = 0x15;
  FUN_01578a60((int)this + 0x168);
  local_4 = CONCAT31(local_4._1_3_,0x16);
  *(undefined1 *)((int)this + 0xc0) = 0;
  *(undefined1 *)((int)this + 0xc1) = 0;
  *(undefined4 *)((int)this + 0xbc) = param_6;
  *(undefined4 *)((int)this + 0xe4) = 0;
  *(undefined4 *)((int)this + 0xe8) = 0;
  *(undefined4 *)((int)this + 0xec) = 0;
  *(undefined4 *)((int)this + 0xf0) = 0;
  *(undefined4 *)((int)this + 0xf4) = 0;
  *(undefined4 *)((int)this + 0xf8) = 0;
  Mercury__unknown_015a4060();
  FUN_01577260(this,(undefined8 *)&DAT_01e91de0,DAT_01e91de0,puVar6);
  iVar5 = param_5;
  puVar6 = *(undefined4 **)(param_5 + 4);
  if (*(undefined4 **)(param_5 + 8) < puVar6) {
    _invalid_parameter_noinfo();
  }
  while( true ) {
    param_4 = *(undefined4 **)(iVar5 + 8);
    if (param_4 < *(undefined4 **)(iVar5 + 4)) {
      _invalid_parameter_noinfo();
    }
    if (puVar6 == param_4) break;
    if (*(undefined4 **)(iVar5 + 8) <= puVar6) {
      _invalid_parameter_noinfo();
    }
    uVar3 = Mercury_Nub_addListeningSocket(this,(undefined1 *)puVar6);
    if ((char)uVar3 == '\0') {
      if (*(undefined4 **)(iVar5 + 8) <= puVar6) {
        _invalid_parameter_noinfo();
      }
      iStack_50 = 0;
      local_4c = 6;
      Mercury__unknown_00a351d0
                (&iStack_50,"%s: Failed to create listening socket on interface %s\n");
    }
    if (*(undefined4 **)(iVar5 + 8) <= puVar6) {
      _invalid_parameter_noinfo();
    }
    puVar6 = puVar6 + 7;
  }
  plVar4 = Mercury__unknown_01578ab0
                     ((void *)((int)this + 0x18),(longlong *)0x1e8480,(int)this + 0x74,0,'\x01');
  *(longlong **)((int)this + 0x128) = plVar4;
  _00___FRawStaticIndexBuffer__vfunc_0();
  FUN_015840d0((void *)(*(int *)((int)this + 0x124) + 8),auStack_48,
               *(undefined4 *)(*(int *)((int)this + 0x124) + 4));
  ExceptionList = pvStack_c;
  return this;
}


/* Mercury debug (Endpoint): Endpoint::findIndicatedInterface: netmask match %s length %s is not
   valid.
    */

undefined4 Mercury_Endpoint(char *param_1,char *param_2)

{
  void *pvVar1;
  undefined1 *puVar2;
  uint uVar3;
  char *pcVar4;
  int iVar5;
  uint uVar6;
  u_long uVar7;
  byte bVar8;
  undefined **ppuVar9;
  char cStack_71;
  ulong local_70;
  int local_6c;
  undefined1 *puStack_64;
  uint uStack_60;
  u_long uStack_5c;
  undefined1 auStack_58 [4];
  uint uStack_54;
  uint uStack_50;
  undefined4 uStack_4c;
  undefined1 auStack_48 [4];
  undefined4 auStack_44 [4];
  undefined4 uStack_34;
  uint uStack_30;
  char local_2c [16];
  undefined1 local_1c;
  void *local_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  pvVar1 = ExceptionList;
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_017942e0;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *param_2 = '\0';
  if ((param_1 == (char *)0x0) || (*param_1 == '\0')) {
    ExceptionList = pvVar1;
    return 0xffffffff;
  }
  local_6c = 0x20;
  strncpy(local_2c,param_1,0x10);
  local_1c = 0;
  local_70 = 0;
  pcVar4 = strchr(param_1,0x2f);
  if ((pcVar4 == (char *)0x0) || (0x10 < (int)pcVar4 - (int)param_1)) {
    iVar5 = Mercury__unknown_015770b0(local_2c,&local_70);
    if (iVar5 == 0) {
      strncpy(param_2,local_2c,0x10);
    }
    else {
      iVar5 = FUN_01584830(param_1,&local_70);
      if (iVar5 != 0) {
        _00___FRawStaticIndexBuffer__vfunc_0();
        ExceptionList = local_c;
        return 0xffffffff;
      }
      local_6c = 0x20;
    }
  }
  else {
    local_2c[(int)pcVar4 - (int)param_1] = '\0';
    iVar5 = FUN_01584830(local_2c,&local_70);
    local_6c = atoi(pcVar4 + 1);
    if (iVar5 != 0 || 0x1f < local_6c - 1U) {
      _00___FRawStaticIndexBuffer__vfunc_0();
      ExceptionList = local_c;
      return 0xffffffff;
    }
  }
  if (*param_2 != '\0') {
    ExceptionList = local_c;
    return 0;
  }
  bVar8 = 0x20 - (char)local_6c;
  uStack_5c = ntohl(local_70);
  uStack_54 = 0;
  uStack_50 = 0;
  uStack_4c = 0;
  iStack_4 = 0;
  if (PTR_DAT_01e91bbc != (undefined *)0x0) {
    ppuVar9 = &PTR_DAT_01e91bbc;
    do {
      pcVar4 = *ppuVar9;
      uStack_30 = 0xf;
      cStack_71 = '\0';
      uStack_34 = 0;
      std::char_traits<char>::assign((char *)auStack_44,&cStack_71);
      uVar6 = std::char_traits<char>::length(pcVar4);
      FUN_004377d0(auStack_48,pcVar4,uVar6);
      iStack_4._0_1_ = 1;
      FUN_0045b4b0(auStack_58,auStack_48);
      iStack_4 = (uint)iStack_4._1_3_ << 8;
      if (0xf < uStack_30) {
                    /* WARNING: Subroutine does not return */
        scalable_free(auStack_44[0]);
      }
      uStack_30 = 0xf;
      cStack_71 = '\0';
      uStack_34 = 0;
      std::char_traits<char>::assign((char *)auStack_44,&cStack_71);
      ppuVar9 = ppuVar9 + 2;
    } while (*ppuVar9 != (undefined *)0x0);
  }
  FUN_00458480(auStack_58,(int *)&puStack_64);
  puVar2 = puStack_64;
  uVar6 = uStack_60;
  while( true ) {
    uVar3 = uStack_50;
    if (uStack_50 < uStack_54) {
      _invalid_parameter_noinfo();
    }
    if ((puVar2 == (undefined1 *)0x0) || (puVar2 != auStack_58)) {
      _invalid_parameter_noinfo();
    }
    if (uVar6 == uVar3) goto LAB_01584bf5;
    puStack_64 = (undefined1 *)0x0;
    if (puVar2 == (undefined1 *)0x0) {
      _invalid_parameter_noinfo();
    }
    if (*(uint *)(puVar2 + 8) <= uVar6) {
      _invalid_parameter_noinfo();
    }
    if (*(uint *)(uVar6 + 0x18) < 0x10) {
      pcVar4 = (char *)(uVar6 + 4);
    }
    else {
      pcVar4 = *(char **)(uVar6 + 4);
    }
    iVar5 = Mercury__unknown_015770b0(pcVar4,(u_long *)&puStack_64);
    if ((iVar5 == 0) &&
       (uVar7 = ntohl((u_long)puStack_64), uVar7 >> (bVar8 & 0x1f) == uStack_5c >> (bVar8 & 0x1f)))
    break;
    if (*(uint *)(puVar2 + 8) <= uVar6) {
      _invalid_parameter_noinfo();
    }
    uVar6 = uVar6 + 0x1c;
  }
  strncpy(param_2,pcVar4,0x10);
LAB_01584bf5:
  if (*param_2 != '\0') {
    iStack_4 = 0xffffffff;
    ZipFileSystem__unknown_0045ad70((int)auStack_58);
    ExceptionList = local_c;
    return 0;
  }
  _00___FRawStaticIndexBuffer__vfunc_0();
  iStack_4 = 0xffffffff;
  ZipFileSystem__unknown_0045ad70((int)auStack_58);
  ExceptionList = local_c;
  return 0xfffffffe;
}


/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* Mercury error in MachineGuard: MachineGuardMessage::sendAndRecv: recvfrom failed (%s)
   
   [String discovery] References: "MGM::sendAndReceiveMGM"
   [String discovery] References: "MGM::sendAndRecv" */

void Mercury_MachineGuard(SOCKET *param_1,int param_2,void *param_3)

{
  char cVar1;
  int *piVar2;
  undefined4 *puVar3;
  _union_1226 in;
  bool bVar4;
  u_short uVar5;
  int iVar6;
  int *piVar7;
  undefined4 *puVar8;
  int *piVar9;
  int *piVar10;
  undefined4 *puVar11;
  int iVar12;
  uint uVar13;
  undefined4 *puVar14;
  int iVar15;
  undefined4 uVar16;
  char local_81cd;
  int *local_81cc;
  undefined1 local_81c8 [4];
  int *local_81c4;
  int local_81c0;
  undefined1 local_81bc [4];
  undefined4 *local_81b8;
  int local_81b4;
  int local_81b0;
  uint local_81ac;
  undefined **local_81a8;
  char local_81a4;
  char *local_81a0;
  char *local_819c;
  _union_1226 local_8198;
  int local_8194;
  undefined4 local_8190 [2];
  int local_8188;
  int local_8184;
  timeval local_8180;
  undefined1 local_8178 [4];
  uint local_8174;
  int local_816c;
  int local_8168;
  undefined1 local_8148 [16];
  undefined4 local_8138 [3];
  undefined4 local_812c [3];
  fd_set local_8120;
  char local_8018 [32764];
  undefined4 uStack_1c;
  void *pvStack_14;
  undefined1 *puStack_10;
  int local_c;
  
  local_c = 0xffffffff;
  puStack_10 = &LAB_017947f4;
  pvStack_14 = ExceptionList;
  uStack_1c = 0x15898e6;
  ExceptionList = &pvStack_14;
  if (param_2 == -1) {
    local_81cd = '\x01';
    ExceptionList = &pvStack_14;
    setsockopt(*param_1,0xffff,0x20,&local_81cd,1);
  }
  FUN_00e9aff0((int)local_81bc);
  local_c = 0;
  FUN_00e9aff0((int)local_81c8);
  local_c._0_1_ = 1;
  iVar12 = 3;
  do {
    uVar16 = 1;
    iVar15 = param_2;
    local_81b0 = iVar12 + -1;
    uVar5 = htons(0x4e36);
    bVar4 = FUN_01588ec0(param_1,uVar5,iVar15,uVar16);
    if (bVar4) {
      local_8120.fd_array[0] = *param_1;
      local_8184 = local_8120.fd_array[0] + 1;
      local_8180.tv_sec = 1;
      local_8180.tv_usec = 0;
      local_8120.fd_count = 1;
      iVar15 = local_8184;
LAB_015899a7:
      iVar6 = select(iVar15,&local_8120,(fd_set *)0x0,(fd_set *)0x0,&local_8180);
      iVar12 = local_81b0;
      if (iVar6 == 1) {
        local_81cc = (int *)0x10;
        iVar12 = recvfrom(*param_1,local_8018,0x8000,0,(sockaddr *)local_8148,(int *)&local_81cc);
        if (-1 < iVar12) {
          local_8198 = (_union_1226)local_8148._4_4_;
        }
        in = local_8198;
        if (iVar12 == -1) {
          piVar7 = _errno();
          strerror(*piVar7);
          _00___FRawStaticIndexBuffer__vfunc_0();
        }
        else {
          local_81cd = '\x01';
          local_81a4 = '\0';
          local_81a0 = local_8018;
          local_819c = local_8018 + iVar12;
          local_81a8 = MemoryIStream::vftable;
          local_c._0_1_ = 3;
          FUN_01589840(local_8178,(int *)&local_81a8);
          local_c._0_1_ = 4;
          local_81ac = local_8174;
          if (local_81a4 == '\0') {
            uVar13 = 0;
            while ((local_816c != 0 && (uVar13 < (uint)(local_8168 - local_816c >> 2)))) {
              piVar7 = *(int **)(local_816c + uVar13 * 4);
              if (*(short *)((int)piVar7 + 6) != *(short *)(local_8194 + 6)) {
                (**(code **)(*piVar7 + 0xc))();
                inet_ntoa((in_addr)in.S_un_b);
                _00___FRawStaticIndexBuffer__vfunc_0();
                local_c._0_1_ = 3;
                Mercury__unknown_015875f0((int)local_8178);
                local_81a8 = MemoryIStream::vftable;
                local_c._0_1_ = 6;
                if (local_81a0 != local_819c) {
                  Mercury__unknown_00a353d0
                            ("MemoryIStream::~MemoryIStream: There are still %d bytes left\n");
                }
                local_c._0_1_ = 1;
                local_81a8 = BinaryIStream::vftable;
                iVar15 = local_8184;
                goto LAB_015899a7;
              }
              if ((*(byte *)((int)piVar7 + 5) & 2) == 0) {
                if ((param_3 != (void *)0x0) &&
                   (local_81cd = FUN_01584f50(param_3,(int)piVar7,in), local_81cd == '\0')) break;
                uVar13 = uVar13 + 1;
              }
              else {
                (**(code **)(*piVar7 + 0xc))();
                inet_ntoa((in_addr)in.S_un_b);
                _00___FRawStaticIndexBuffer__vfunc_0();
                uVar13 = uVar13 + 1;
              }
            }
            FUN_011783d0(local_81bc,local_812c,&local_8198.S_addr);
            piVar7 = (int *)local_81c4[1];
            cVar1 = *(char *)((int)piVar7 + 0x11);
            piVar2 = piVar7;
            piVar10 = local_81c4;
            while (cVar1 == '\0') {
              if ((uint)in < (uint)piVar2[3]) {
                piVar9 = (int *)*piVar2;
                piVar10 = piVar2;
              }
              else {
                piVar9 = (int *)piVar2[2];
              }
              piVar2 = piVar9;
              cVar1 = *(char *)((int)piVar9 + 0x11);
            }
            cVar1 = *(char *)((int)piVar7 + 0x11);
            piVar2 = local_81c4;
            while (piVar9 = piVar7, cVar1 == '\0') {
              if ((uint)piVar9[3] < (uint)in) {
                piVar7 = (int *)piVar9[2];
                piVar9 = piVar2;
              }
              else {
                piVar7 = (int *)*piVar9;
              }
              cVar1 = *(char *)((int)piVar7 + 0x11);
              piVar2 = piVar9;
            }
            local_81cc = (int *)0x0;
            Mercury__unknown_00d09060
                      ((int)local_81c8,(int)piVar2,(int)local_81c8,(int)piVar10,(int *)&local_81cc);
            if (local_81cc != (int *)0x0) {
              piVar7 = (int *)local_81c4[1];
              cVar1 = *(char *)((int)piVar7 + 0x11);
              piVar2 = piVar7;
              local_81cc = local_81c4;
              while (cVar1 == '\0') {
                if ((uint)in < (uint)piVar2[3]) {
                  piVar10 = (int *)*piVar2;
                  local_81cc = piVar2;
                }
                else {
                  piVar10 = (int *)piVar2[2];
                }
                piVar2 = piVar10;
                cVar1 = *(char *)((int)piVar10 + 0x11);
              }
              cVar1 = *(char *)((int)piVar7 + 0x11);
              piVar2 = local_81c4;
              while (piVar10 = piVar7, cVar1 == '\0') {
                if ((uint)piVar10[3] < (uint)local_8198) {
                  piVar7 = (int *)piVar10[2];
                  piVar10 = piVar2;
                }
                else {
                  piVar7 = (int *)*piVar10;
                }
                cVar1 = *(char *)((int)piVar7 + 0x11);
                piVar2 = piVar10;
              }
              local_8188 = 0;
              Mercury__unknown_00d09060
                        ((int)local_81c8,(int)piVar2,(int)local_81c8,(int)local_81cc,&local_8188);
              Mercury__unknown_00ebfb10
                        (local_81c8,local_8190,local_81c8,piVar2,local_81c8,local_81cc);
            }
            puVar8 = (undefined4 *)local_81b8[1];
            cVar1 = *(char *)((int)puVar8 + 0x11);
            puVar3 = puVar8;
            puVar14 = local_81b8;
            while (cVar1 == '\0') {
              if (local_81ac < (uint)puVar3[3]) {
                puVar11 = (undefined4 *)*puVar3;
                puVar14 = puVar3;
              }
              else {
                puVar11 = (undefined4 *)puVar3[2];
              }
              puVar3 = puVar11;
              cVar1 = *(char *)((int)puVar11 + 0x11);
            }
            cVar1 = *(char *)((int)puVar8 + 0x11);
            puVar3 = local_81b8;
            while (puVar11 = puVar8, cVar1 == '\0') {
              if ((uint)puVar11[3] < local_81ac) {
                puVar8 = (undefined4 *)puVar11[2];
                puVar11 = puVar3;
              }
              else {
                puVar8 = (undefined4 *)*puVar11;
              }
              cVar1 = *(char *)((int)puVar8 + 0x11);
              puVar3 = puVar11;
            }
            local_81cc = (int *)0x0;
            Mercury__unknown_00d09060
                      ((int)local_81bc,(int)puVar3,(int)local_81bc,(int)puVar14,(int *)&local_81cc);
            if ((local_81cc == (int *)0x0) && (local_81ac != 0)) {
              FUN_011783d0(local_81c8,local_8138,&local_81ac);
            }
            if (((local_81cd == '\0') || (param_2 != -1)) ||
               ((local_81b4 != 0 && (local_81c0 == 0)))) {
              local_c._0_1_ = 3;
              Mercury__unknown_015875f0((int)local_8178);
              local_81a8 = MemoryIStream::vftable;
              local_c._0_1_ = 7;
              if (local_81a0 != local_819c) {
                Mercury__unknown_00a353d0
                          ("MemoryIStream::~MemoryIStream: There are still %d bytes left\n");
              }
              local_81a8 = BinaryIStream::vftable;
              local_c = (uint)local_c._1_3_ << 8;
              Mercury__unknown_00ebfb10
                        (local_81c8,local_8190,local_81c8,(int *)*local_81c4,local_81c8,local_81c4);
                    /* WARNING: Subroutine does not return */
              scalable_free(local_81c4);
            }
            local_c._0_1_ = 3;
            Mercury__unknown_015875f0((int)local_8178);
            local_c._0_1_ = 1;
            FUN_0157af80(&local_81a8);
            iVar15 = local_8184;
          }
          else {
            local_c._0_1_ = 3;
            Mercury__unknown_015875f0((int)local_8178);
            local_81a8 = MemoryIStream::vftable;
            local_c._0_1_ = 5;
            if (local_81a0 != local_819c) {
              Mercury__unknown_00a353d0
                        ("MemoryIStream::~MemoryIStream: There are still %d bytes left\n");
            }
            local_c._0_1_ = 1;
            local_81a8 = BinaryIStream::vftable;
            iVar15 = local_8184;
          }
        }
        goto LAB_015899a7;
      }
    }
    else {
      _00___FRawStaticIndexBuffer__vfunc_0();
      iVar12 = iVar12 + -1;
    }
    if (iVar12 == 0) {
      _00___FRawStaticIndexBuffer__vfunc_0();
      local_c._0_1_ = 0;
      Mercury__unknown_00ebfb10
                (local_81c8,local_8190,local_81c8,(int *)*local_81c4,local_81c8,local_81c4);
                    /* WARNING: Subroutine does not return */
      scalable_free(local_81c4);
    }
  } while( true );
}


/* WARNING: Removing unreachable block (ram,0x0158aeb6) */
/* WARNING: Removing unreachable block (ram,0x0158aedb) */
/* Mercury debug string: Mercury::InterfaceElement::compressLength( %s ): length %d exceeds maximum
   of length format %d
    */

undefined4 __thiscall
Mercury_InterfaceElement_compressLength(void *this,undefined4 *param_1,int *param_2,int *param_3)

{
  undefined1 uVar1;
  undefined4 *puVar2;
  int iVar3;
  int *piVar4;
  undefined1 *puVar5;
  uint uVar6;
  int *piVar7;
  undefined4 *this_00;
  undefined4 *unaff_retaddr;
  undefined4 *puStack_58;
  int iStack_54;
  int iStack_50;
  int iStack_4c;
  int iStack_48;
  int iStack_44;
  int iStack_40;
  int *piStack_3c;
  int iStack_38;
  int *piStack_34;
  int iStack_30;
  int iStack_2c;
  undefined4 *puStack_28;
  undefined1 *apuStack_24 [2];
  int aiStack_1c [3];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  puVar2 = param_1;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01794920;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  _snprintf(&DAT_01f125a8,0x100,"%s/%d");
  _00___FRawStaticIndexBuffer__vfunc_0();
  iVar3 = 1;
  if (0 < *(int *)((int)this + 4)) {
    do {
      *(undefined1 *)(iVar3 + (int)param_1) = 0xff;
      iVar3 = iVar3 + 1;
    } while (iVar3 <= *(int *)((int)this + 4));
  }
  param_1 = (undefined4 *)param_3[1];
  if (param_1 != (undefined4 *)0x0) {
    piStack_3c = param_1 + 1;
    LOCK();
    iStack_38 = *piStack_3c;
    *piStack_3c = *piStack_3c + 1;
    UNLOCK();
  }
  uStack_4 = 0;
  this_00 = param_1;
  while( true ) {
    if (this_00 == (undefined4 *)0x0) {
      _snprintf(&DAT_01f125a8,0x100,"%s/%d");
      aiStack_1c[0] = 0;
      aiStack_1c[1] = 6;
      Mercury__unknown_00a351d0
                (aiStack_1c,
                 "Mercury::InterfaceElement::compressLength( %s ): data not in any packets\n");
      ExceptionList = pvStack_c;
      return 0xffffffff;
    }
    if (((undefined4 *)((int)this_00 + 0x55U) <= puVar2) &&
       (puVar2 < (undefined4 *)(this_00[9] + 0x54 + (int)this_00))) break;
    piVar4 = Mercury__unknown_0158a850(this_00,(int *)&puStack_58);
    uStack_4._0_1_ = 1;
    if (this_00 != (undefined4 *)*piVar4) {
      if (this_00 != (undefined4 *)0x0) {
        piStack_34 = this_00 + 1;
        LOCK();
        iStack_54 = *piStack_34;
        *piStack_34 = *piStack_34 + -1;
        UNLOCK();
        if (iStack_54 == 1 || iStack_54 + -1 < 0) {
          (**(code **)*param_1)();
        }
      }
      this_00 = (undefined4 *)*piVar4;
      param_1 = this_00;
      if (this_00 != (undefined4 *)0x0) {
        piStack_34 = this_00 + 1;
        LOCK();
        iStack_30 = *piStack_34;
        *piStack_34 = *piStack_34 + 1;
        UNLOCK();
      }
    }
    uStack_4 = (uint)uStack_4._1_3_ << 8;
    if (puStack_58 != (undefined4 *)0x0) {
      piStack_34 = puStack_58 + 1;
      LOCK();
      iStack_50 = *piStack_34;
      *piStack_34 = *piStack_34 + -1;
      UNLOCK();
      this_00 = param_1;
      if ((iStack_50 == 1 || iStack_50 + -1 < 0) && (puStack_58 != (undefined4 *)0x0)) {
        (**(code **)*puStack_58)();
      }
    }
  }
  puStack_28 = (undefined4 *)&stack0xffffff90;
  piVar4 = this_00 + 1;
  LOCK();
  iStack_2c = *piVar4;
  *piVar4 = *piVar4 + 1;
  UNLOCK();
  uStack_4 = uStack_4 & 0xffffff00;
  piStack_34 = piVar4;
  FUN_0158aa90(apuStack_24,this_00,puVar2);
  uStack_4 = CONCAT31(uStack_4._1_3_,3);
  InterfaceElement__unknown_0158ab40(apuStack_24,(undefined4 *)(*(int *)((int)this + 4) + 1));
  iVar3 = (**(code **)(*param_3 + 4))();
  iStack_50 = 0;
  piVar7 = param_2;
  do {
    uVar1 = *apuStack_24[0];
    puVar5 = (undefined1 *)(**(code **)(*param_2 + 0x14))(iVar3);
    *puVar5 = uVar1;
    *apuStack_24[0] = (char)piVar7;
    piVar7 = (int *)((uint)piVar7 >> 8);
    iVar3 = iVar3 + 1;
    uVar6 = InterfaceElement__unknown_0158ab40(&puStack_28,(undefined4 *)0x1);
    puVar2 = puStack_28;
    if ((char)uVar6 == '\0') {
      _snprintf(&DAT_01f125a8,0x100,"%s/%d",*(undefined4 *)((int)this + 8));
      aiStack_1c[1] = 0;
      aiStack_1c[2] = 6;
      Mercury__unknown_00a351d0
                (aiStack_1c + 1,
                 "Mercury::InterfaceElement::compressLength( %s ): head not in packets.\n");
      puVar2 = puStack_28;
      puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
      if (puStack_28 != (undefined4 *)0x0) {
        piVar7 = puStack_28 + 1;
        LOCK();
        iVar3 = *piVar7;
        *piVar7 = *piVar7 + -1;
        UNLOCK();
        iStack_4c = iVar3;
        if ((iVar3 == 1 || iVar3 + -1 < 0) && (puVar2 != (undefined4 *)0x0)) {
          (**(code **)*puVar2)();
        }
      }
      puStack_8 = (undefined1 *)0xffffffff;
      LOCK();
      iStack_48 = *piVar4;
      *piVar4 = *piVar4 + -1;
      UNLOCK();
      if (iStack_48 == 1 || iStack_48 + -1 < 0) {
        (**(code **)*unaff_retaddr)();
      }
      ExceptionList = pvStack_10;
      return 0xffffffff;
    }
    iStack_50 = iStack_50 + 1;
  } while (iStack_50 < 4);
  puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
  if (puStack_28 != (undefined4 *)0x0) {
    piVar7 = puStack_28 + 1;
    LOCK();
    iVar3 = *piVar7;
    *piVar7 = *piVar7 + -1;
    UNLOCK();
    iStack_44 = iVar3;
    if ((iVar3 == 1 || iVar3 + -1 < 0) && (puVar2 != (undefined4 *)0x0)) {
      (**(code **)*puVar2)();
    }
  }
  puStack_8 = (undefined1 *)0xffffffff;
  LOCK();
  iStack_40 = *piVar4;
  *piVar4 = *piVar4 + -1;
  UNLOCK();
  if (iStack_40 == 1 || iStack_40 + -1 < 0) {
    (**(code **)*unaff_retaddr)();
  }
  ExceptionList = pvStack_10;
  return 0;
}


/* Mercury debug string: Mercury::InterfaceElement::expandLength( %s ): Overflow in calculating
   length of variable message!
    */

undefined4 * __thiscall Mercury_InterfaceElement_expandLength(void *this,int param_1)

{
  undefined4 uVar1;
  undefined4 *extraout_ECX;
  undefined4 *puVar2;
  int aiStack_14 [2];
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_017949a0;
  local_c = ExceptionList;
  local_4 = 0;
  if (*(char *)((int)this + 1) == '\0') {
    puVar2 = *(undefined4 **)((int)this + 4);
    ExceptionList = &local_c;
    goto LAB_0158b91d;
  }
  if (*(char *)((int)this + 1) != '\x01') {
    ExceptionList = &local_c;
    _snprintf(&DAT_01f125a8,0x100,"%s/%d",*(undefined4 *)((int)this + 8));
    _00___FRawStaticIndexBuffer__vfunc_0();
    goto LAB_0158b834;
  }
  puVar2 = (undefined4 *)0x0;
  switch(*(undefined4 *)((int)this + 4)) {
  case 1:
    puVar2 = (undefined4 *)(uint)*(byte *)(param_1 + 1);
    break;
  case 2:
    puVar2 = (undefined4 *)(uint)*(ushort *)(param_1 + 1);
    break;
  case 3:
    puVar2 = (undefined4 *)(uint)*(uint3 *)(param_1 + 1);
    break;
  case 4:
    puVar2 = *(undefined4 **)(param_1 + 1);
    break;
  default:
    ExceptionList = &local_c;
    _snprintf(&DAT_01f125a8,0x100,"%s/%d",*(undefined4 *)((int)this + 8));
    aiStack_14[0] = 0;
    aiStack_14[1] = 6;
    Mercury__unknown_00a351d0
              (aiStack_14,
               "InterfaceElement::expandLength( %s ): Unhandled variable message length: %d\n");
    goto LAB_0158b8c0;
  }
  ExceptionList = &local_c;
  if ((int)puVar2 < 0) {
    ExceptionList = &local_c;
    _snprintf(&DAT_01f125a8,0x100,"%s/%d",*(undefined4 *)((int)this + 8));
    _00___FRawStaticIndexBuffer__vfunc_0();
LAB_0158b834:
    local_4 = 0xffffffff;
    Mercury__unknown_0157df90((int *)&stack0x00000008);
    ExceptionList = local_c;
    return (undefined4 *)0xffffffff;
  }
LAB_0158b8c0:
  uVar1 = FUN_01578e20(this,(int)puVar2);
  if ((char)uVar1 != '\0') {
    local_4 = 0xffffffff;
    Mercury__unknown_0157df90((int *)&stack0x00000008);
    ExceptionList = local_c;
    return puVar2;
  }
  puVar2 = extraout_ECX;
  Mercury__unknown_01576d50((int *)&stack0xffffffdc,(int)&stack0x00000008,(int *)&stack0x00000008);
  local_4 = local_4 & 0xffffff00;
  puVar2 = InterfaceElement_expandLength_1(this,param_1,puVar2);
LAB_0158b91d:
  local_4 = 0xffffffff;
  Mercury__unknown_0157df90((int *)&stack0x00000008);
  ExceptionList = local_c;
  return puVar2;
}


/* Mercury debug (Channel): Channel::~Channel( %s ): Forgetting %d unprocessed packets in the
   fragment chain
   [String discovery] References: "ChannelInternal::resetRemotePart" */

void __thiscall Mercury_Channel_18(void *this,char param_1)

{
  int *piVar1;
  int *piVar2;
  int iVar3;
  undefined4 *puVar4;
  uint uVar5;
  
  if (*(int *)((int)this + 0x124) != 0) {
    Mercury__unknown_0158a8e0(*(void **)(*(int *)((int)this + 0x124) + 0x10));
    if (param_1 != '\0') {
      std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::c_str
                ((basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> *)
                 ((int)this + 0x13c));
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
    iVar3 = *(int *)((int)this + 0x124);
    if (iVar3 != 0) {
      Mercury__unknown_0157f050(iVar3);
                    /* WARNING: Subroutine does not return */
      scalable_free(iVar3);
    }
    *(undefined4 *)((int)this + 0x124) = 0;
  }
  if (*(int *)((int)this + 0x48) != 0) {
    if (param_1 != '\0') {
      std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::c_str
                ((basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> *)
                 ((int)this + 0x80));
      _00___FRawStaticIndexBuffer__vfunc_0();
    }
    uVar5 = 0;
    if (*(int *)((int)this + 0x44) != -1) {
      do {
        if (*(int *)((int)this + 0x48) == 0) break;
        if (*(int *)(*(int *)((int)this + 0x40) + (*(uint *)((int)this + 0x44) & uVar5) * 4) != 0) {
          piVar2 = (int *)(*(int *)((int)this + 0x40) + (*(uint *)((int)this + 0x44) & uVar5) * 4);
          puVar4 = (undefined4 *)*piVar2;
          if (puVar4 != (undefined4 *)0x0) {
            piVar1 = puVar4 + 1;
            LOCK();
            iVar3 = *piVar1;
            *piVar1 = *piVar1 + -1;
            UNLOCK();
            if ((iVar3 == 1 || iVar3 + -1 < 0) && (puVar4 != (undefined4 *)0x0)) {
              (**(code **)*puVar4)(1);
            }
          }
          *piVar2 = 0;
          *(int *)((int)this + 0x48) = *(int *)((int)this + 0x48) + -1;
        }
        uVar5 = uVar5 + 1;
      } while (uVar5 < *(int *)((int)this + 0x44) + 1U);
    }
  }
  if (*(int *)((int)this + 0x120) != 0) {
    Mercury__unknown_01578670(*(int *)((int)this + 0x120));
    *(undefined4 *)((int)this + 0x120) = 0;
  }
  *(undefined4 *)((int)this + 0x50) = 0x10000000;
  return;
}


/* Mercury debug (Channel): Caught std::exception %s while trying to destruct local part of
   InnerChannel on  */

undefined * Mercury_Channel_16(void)

{
  char *pcVar1;
  int unaff_EBP;
  
  pcVar1 = Mercury__unknown_015847f0((void *)(*(int *)(unaff_EBP + -0x14) + 0x114));
  (**(code **)(**(int **)(unaff_EBP + -0x1c) + 4))(pcVar1);
  _00___FRawStaticIndexBuffer__vfunc_0();
  *(undefined4 *)(unaff_EBP + -4) = 7;
  return &DAT_0158d264;
}


/* Mercury debug (Channel): Caught std::exception %s while trying to destruct remote part of
   InnerChannel on */

undefined * Mercury_Channel_17(void)

{
  char *pcVar1;
  int unaff_EBP;
  
  pcVar1 = Mercury__unknown_015847f0((void *)(*(int *)(unaff_EBP + -0x14) + 0x114));
  (**(code **)(**(int **)(unaff_EBP + -0x24) + 4))(pcVar1);
  _00___FRawStaticIndexBuffer__vfunc_0();
  *(undefined4 *)(unaff_EBP + -4) = 7;
  return &DAT_0158d30d;
}


