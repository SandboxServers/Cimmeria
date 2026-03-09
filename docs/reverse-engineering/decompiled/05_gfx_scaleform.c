/*
 * SGW.exe Decompilation - 05_gfx_scaleform
 * Classes: 157
 * Stargate Worlds Client
 */


/* ========== FGFxAllocator.c ========== */

/*
 * SGW.exe - FGFxAllocator (4 functions)
 */

/* [VTable] FGFxAllocator virtual function #2
   VTable: vtable_FGFxAllocator at 01960fb0 */

void FGFxAllocator__vfunc_2(void)

{
  if (DAT_01ea5778 == (int *)0x0) {
    FUN_00416660();
  }
                    /* WARNING: Could not recover jumptable at 0x00af1a6a. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*DAT_01ea5778 + 0xc))();
  return;
}


/* [VTable] FGFxAllocator virtual function #1
   VTable: vtable_FGFxAllocator at 01960fb0 */

void FGFxAllocator__vfunc_1(undefined4 param_1)

{
  if (DAT_01ea5778 == (int *)0x0) {
    FUN_00416660();
  }
  (**(code **)(*DAT_01ea5778 + 4))(param_1,8);
  return;
}


/* [VTable] FGFxAllocator virtual function #3
   VTable: vtable_FGFxAllocator at 01960fb0 */

void FGFxAllocator__vfunc_3(undefined4 param_1,undefined4 param_2)

{
  if (DAT_01ea5778 == (int *)0x0) {
    FUN_00416660();
  }
  (**(code **)(*DAT_01ea5778 + 8))(param_1,param_2,8);
  return;
}


/* Also in vtable: FGFxAllocator (slot 0) */

undefined4 * __thiscall FGFxAllocator__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  uint uVar2;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_016da868;
  local_c = ExceptionList;
  uVar2 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  *(undefined ***)this = GAllocator::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar2);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== FGFxEngine.c ========== */

/*
 * SGW.exe - FGFxEngine (1 functions)
 */

/* Also in vtable: FGFxEngine (slot 0) */

undefined4 * __thiscall FGFxEngine__vfunc_0(void *this,byte param_1)

{
  FUN_00af0bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FGFxEngineBase.c ========== */

/*
 * SGW.exe - FGFxEngineBase (1 functions)
 */

/* Also in vtable: FGFxEngineBase (slot 0) */

undefined4 * __thiscall FGFxEngineBase__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FGFxEngineBase::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FGFxFile.c ========== */

/*
 * SGW.exe - FGFxFile (16 functions)
 */

/* [VTable] FGFxFile virtual function #1
   VTable: vtable_FGFxFile at 019633bc */

int __fastcall FGFxFile__vfunc_1(int param_1)

{
  return param_1 + 0x1c;
}


/* [VTable] FGFxFile virtual function #2
   VTable: vtable_FGFxFile at 019633bc */

bool __fastcall FGFxFile__vfunc_2(int param_1)

{
  return *(int *)(param_1 + 0x10) != 0;
}


/* [VTable] FGFxFile virtual function #3
   VTable: vtable_FGFxFile at 019633bc */

void __fastcall FGFxFile__vfunc_3(int *param_1)

{
                    /* WARNING: Could not recover jumptable at 0x00aff2a5. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*param_1 + 8))();
  return;
}


/* [VTable] FGFxFile virtual function #4
   VTable: vtable_FGFxFile at 019633bc */

undefined4 __fastcall FGFxFile__vfunc_4(int param_1)

{
  return *(undefined4 *)(param_1 + 0x18);
}


/* [VTable] FGFxFile virtual function #5
   VTable: vtable_FGFxFile at 019633bc */

void FGFxFile__vfunc_5(void)

{
  return;
}


/* [VTable] FGFxFile virtual function #6
   VTable: vtable_FGFxFile at 019633bc */

undefined4 __fastcall FGFxFile__vfunc_6(int param_1)

{
  return *(undefined4 *)(param_1 + 0x14);
}


/* [VTable] FGFxFile virtual function #7
   VTable: vtable_FGFxFile at 019633bc */

void FGFxFile__vfunc_7(void)

{
  return;
}


/* [VTable] FGFxFile virtual function #8
   VTable: vtable_FGFxFile at 019633bc */

undefined4 __fastcall FGFxFile__vfunc_8(int param_1)

{
  return *(undefined4 *)(param_1 + 0x5c);
}


/* [VTable] FGFxFile virtual function #9
   VTable: vtable_FGFxFile at 019633bc */

size_t __thiscall FGFxFile__vfunc_9(void *this,void *param_1,size_t param_2)

{
  int iVar1;
  
  if ((-1 < (int)param_2) && (param_1 != (void *)0x0)) {
    iVar1 = *(int *)((int)this + 0x18);
    if (*(int *)((int)this + 0x14) <= (int)(iVar1 + param_2)) {
      param_2 = (*(int *)((int)this + 0x14) - iVar1) - 1;
    }
    memcpy((void *)(*(int *)((int)this + 0x10) + iVar1),param_1,param_2);
    *(int *)((int)this + 0x18) = *(int *)((int)this + 0x18) + param_2;
    return param_2;
  }
  return 0xffffffff;
}


/* [VTable] FGFxFile virtual function #10
   VTable: vtable_FGFxFile at 019633bc */

size_t __thiscall FGFxFile__vfunc_10(void *this,void *param_1,size_t param_2)

{
  int iVar1;
  
  if ((int)param_2 < 0) {
    return 0xffffffff;
  }
  iVar1 = *(int *)((int)this + 0x18);
  if (*(int *)((int)this + 0x14) <= (int)(iVar1 + param_2)) {
    param_2 = (*(int *)((int)this + 0x14) - iVar1) - 1;
  }
  memcpy(param_1,(void *)(*(int *)((int)this + 0x10) + iVar1),param_2);
  *(int *)((int)this + 0x18) = *(int *)((int)this + 0x18) + param_2;
  return param_2;
}


/* [VTable] FGFxFile virtual function #11
   VTable: vtable_FGFxFile at 019633bc */

int __thiscall FGFxFile__vfunc_11(void *this,int param_1)

{
  int iVar1;
  
  if (param_1 < 0) {
    return -1;
  }
  iVar1 = *(int *)((int)this + 0x18);
  if (*(int *)((int)this + 0x14) <= iVar1 + param_1) {
    param_1 = (*(int *)((int)this + 0x14) - iVar1) + -1;
  }
  *(int *)((int)this + 0x18) = iVar1 + param_1;
  return param_1;
}


/* [VTable] FGFxFile virtual function #12
   VTable: vtable_FGFxFile at 019633bc */

int __fastcall FGFxFile__vfunc_12(int param_1)

{
  return *(int *)(param_1 + 0x14) - *(int *)(param_1 + 0x18);
}


/* [VTable] FGFxFile virtual function #14
   VTable: vtable_FGFxFile at 019633bc */

int __thiscall FGFxFile__vfunc_14(void *this,int param_1,int param_2)

{
  int iVar1;
  
  if (param_2 == 0) {
    if (*(int *)((int)this + 0x14) <= param_1) {
      iVar1 = *(int *)((int)this + 0x14) + -1;
      *(int *)((int)this + 0x18) = iVar1;
      return iVar1;
    }
  }
  else {
    if (param_2 != 1) {
      if (param_2 == 2) {
        if (param_1 < *(int *)((int)this + 0x14)) {
          iVar1 = (*(int *)((int)this + 0x14) - param_1) + -1;
          *(int *)((int)this + 0x18) = iVar1;
          return iVar1;
        }
        *(undefined4 *)((int)this + 0x18) = 0;
        return 0;
      }
      goto LAB_00aff443;
    }
    param_1 = *(int *)((int)this + 0x18) + param_1;
    if (*(int *)((int)this + 0x14) <= param_1) {
      iVar1 = *(int *)((int)this + 0x14) + -1;
      *(int *)((int)this + 0x18) = iVar1;
      return iVar1;
    }
  }
  *(int *)((int)this + 0x18) = param_1;
LAB_00aff443:
  return *(int *)((int)this + 0x18);
}


/* [VTable] FGFxFile virtual function #15
   VTable: vtable_FGFxFile at 019633bc */

void __thiscall
FGFxFile__vfunc_15(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  (**(code **)(*(int *)this + 0x38))(param_1,param_3);
  return;
}


/* Also in vtable: FGFxFile (slot 0) */

GRefCountBaseImpl * __thiscall FGFxFile__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016dbff8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = FGFxFile::vftable;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}


/* [VTable] FGFxFile virtual function #16
   VTable: vtable_FGFxFile at 019633bc */

undefined1 FGFxFile__vfunc_16(void)

{
  return 1;
}




/* ========== FGFxFileOpener.c ========== */

/*
 * SGW.exe - FGFxFileOpener (2 functions)
 */

/* [VTable] FGFxFileOpener virtual function #1
   VTable: vtable_FGFxFileOpener at 01960c20 */

GRefCountBaseImpl * FGFxFileOpener__vfunc_1(GFile *param_1)

{
  bool bVar1;
  uint uVar2;
  void *this;
  GBufferedFile *this_00;
  GSysFile *this_01;
  GRefCountBaseImpl *pGVar3;
  undefined **local_18;
  int local_14;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016da192;
  local_c = ExceptionList;
  uVar2 = DAT_01e7f9a0 ^ (uint)&stack0xffffffcc;
  ExceptionList = &local_c;
  bVar1 = false;
  if (param_1 == (GFile *)0x0) {
    FUN_00486000("ppath",".\\Src\\GFxUIEngine.cpp",0x43);
  }
  if ((*param_1 == (GFile)0x2f) || ((param_1[1] == (GFile)0x3a && (param_1[2] == (GFile)0x2f)))) {
    this_00 = (GBufferedFile *)FUN_00455b00(0x30);
    if (this_00 != (GBufferedFile *)0x0) {
      *(undefined4 *)(this_00 + 4) = 0x56471e89;
      *(undefined4 *)(this_00 + 8) = 0x9fe1234a;
    }
    local_4 = 0;
    if (this_00 == (GBufferedFile *)0x0) {
      pGVar3 = (GRefCountBaseImpl *)0x0;
    }
    else {
      this_01 = (GSysFile *)FUN_00455b00(0x14);
      if (this_01 != (GSysFile *)0x0) {
        *(undefined4 *)(this_01 + 4) = 0x56471e89;
        *(undefined4 *)(this_01 + 8) = 0x9fe1234a;
      }
      local_4._0_1_ = 1;
      if (this_01 == (GSysFile *)0x0) {
        param_1 = (GFile *)0x0;
      }
      else {
        param_1 = (GFile *)GSysFile::GSysFile(this_01,(char *)param_1,1,0x1b6);
      }
      local_4 = CONCAT31(local_4._1_3_,2);
      bVar1 = true;
      pGVar3 = (GRefCountBaseImpl *)GBufferedFile::GBufferedFile(this_00,param_1);
    }
    local_4 = 0xffffffff;
    if ((bVar1) && (param_1 != (GFile *)0x0)) {
      (**(code **)(*(int *)(param_1 + 8) + 4))(uVar2);
    }
  }
  else {
    pGVar3 = (GRefCountBaseImpl *)0x0;
    FUN_00aef950(&local_18,(LPCSTR)param_1);
    local_4 = 4;
    if (local_14 == 0) {
      local_18 = &PTR_017f8e10;
    }
    if (DAT_01ef0a34 == (undefined4 *)0x0) {
      DAT_01ef0a34 = FUN_00af16e0();
      FUN_00af17b0();
    }
    uVar2 = FUN_004a8e10(DAT_01ef0a34,(undefined4 *)0x0,local_18,(wchar_t *)0x0,0,(int *)0x0,1);
    if (uVar2 != 0) {
      this = (void *)FUN_00455b00(0x60);
      if (this != (void *)0x0) {
        *(undefined4 *)((int)this + 4) = 0x56471e89;
        *(undefined4 *)((int)this + 8) = 0x9fe1234a;
      }
      local_4 = CONCAT31(local_4._1_3_,5);
      if (this == (void *)0x0) {
        pGVar3 = (GRefCountBaseImpl *)0x0;
      }
      else {
        pGVar3 = FUN_00aff5a0(this,(char *)param_1,*(int *)(uVar2 + 0x3c),*(int *)(uVar2 + 0x40));
      }
    }
    local_4 = 0xffffffff;
    FMapPackageFileCache__unknown_00d08060((int *)&local_18);
  }
  ExceptionList = local_c;
  return pGVar3;
}


/* Also in vtable: FGFxFileOpener (slot 0) */

GRefCountBaseImpl * __thiscall FGFxFileOpener__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016da3f0;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = GFxState::vftable;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== FGFxImageCreator.c ========== */

/*
 * SGW.exe - FGFxImageCreator (2 functions)
 */

/* Also in vtable: FGFxImageCreator (slot 0) */

GRefCountBaseImpl * __thiscall FGFxImageCreator__vfunc_0(void *this,byte param_1)

{
  FUN_00aefea0(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}


/* [VTable] FGFxImageCreator virtual function #1
   VTable: vtable_FGFxImageCreator at 01960c54 */

GRefCountBaseImpl * __thiscall FGFxImageCreator__vfunc_1(void *this,int *param_1)

{
  int iVar1;
  bool bVar2;
  bool bVar3;
  GRefCountBaseImpl *pGVar4;
  undefined4 *puVar5;
  void *pvVar6;
  UINT *pUVar7;
  LONG LVar8;
  int local_ec;
  undefined **local_e8;
  int local_e4;
  int local_dc [3];
  int local_d0 [3];
  int local_c4 [3];
  int local_b8 [3];
  int local_ac [3];
  int local_a0 [3];
  undefined1 local_94 [4];
  undefined1 auStack_90 [128];
  undefined1 *puStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016da629;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  bVar3 = false;
  bVar2 = false;
  iVar1 = *param_1;
  if (iVar1 == 0) {
    pGVar4 = (GRefCountBaseImpl *)FUN_00455b00(0x28);
    if (pGVar4 != (GRefCountBaseImpl *)0x0) {
      *(undefined4 *)(pGVar4 + 4) = 0x56471e89;
      *(undefined4 *)(pGVar4 + 8) = 0x9fe1234a;
    }
    local_4 = 0;
  }
  else {
    if (iVar1 == 1) {
      pGVar4 = (GRefCountBaseImpl *)FUN_00455b00(0x28);
      if (pGVar4 != (GRefCountBaseImpl *)0x0) {
        *(undefined4 *)(pGVar4 + 4) = 0x56471e89;
        *(undefined4 *)(pGVar4 + 8) = 0x9fe1234a;
      }
      local_4 = 1;
      if (pGVar4 == (GRefCountBaseImpl *)0x0) {
        ExceptionList = local_c;
        return (GRefCountBaseImpl *)0x0;
      }
      pGVar4 = FUN_013cf7e0(pGVar4,param_1[3],(GRefCountBaseImpl)0x0);
      ExceptionList = local_c;
      return pGVar4;
    }
    if (iVar1 == 2) {
      puVar5 = FUN_0047d9d0(local_a0,(LPCSTR)(*(int *)(param_1[3] + 0x18) + 0xc));
      local_4 = 2;
      FMapPackageFileCache__unknown_004d39f0(local_dc,puVar5);
      local_4._0_1_ = 4;
      FUN_0041b420(local_a0);
      puVar5 = FUN_00488b90(local_dc,local_c4,1);
      local_4._0_1_ = 5;
      FMapPackageFileCache__unknown_004d39f0(local_d0,puVar5);
      local_4._0_1_ = 7;
      FUN_0041b420(local_c4);
      puVar5 = FUN_0041aab0(local_ac,&DAT_01960d08);
      local_4._0_1_ = 8;
      pvVar6 = FUN_0041b600((void *)((int)this + 0x18),local_b8,puVar5);
      local_4._0_1_ = 9;
      FUN_0041b600(pvVar6,&local_e8,local_d0);
      local_4._0_1_ = 0xb;
      FUN_0041b420(local_b8);
      local_4._0_1_ = 0xc;
      FUN_0041b420(local_ac);
      pvVar6 = (void *)FUN_00455b00(0x24);
      if (pvVar6 != (void *)0x0) {
        *(undefined4 *)((int)pvVar6 + 4) = 0x56471e89;
        *(undefined4 *)((int)pvVar6 + 8) = 0x9fe1234a;
      }
      local_4._0_1_ = 0xd;
      if (pvVar6 == (void *)0x0) {
        pGVar4 = (GRefCountBaseImpl *)0x0;
      }
      else {
        if (local_e4 == 0) {
          local_e8 = &PTR_017f8e10;
        }
        pUVar7 = FUN_00423f40(local_94,(LPCWSTR)local_e8);
        local_4 = CONCAT31(local_4._1_3_,0xe);
        FID__unknown_013cfde0(&local_ec,(char *)pUVar7[0x21]);
        local_4 = 0xf;
        bVar3 = true;
        bVar2 = true;
        pGVar4 = FUN_00af06c0(pvVar6,&local_ec,(uint)*(ushort *)(param_1[3] + 0x1c),
                              (uint)*(ushort *)(param_1[3] + 0x1e));
      }
      local_4 = 0x10;
      if ((bVar2) && (LVar8 = InterlockedExchangeAdd((LONG *)(local_ec + 8),-1), LVar8 == 1)) {
        FUN_00455b50(local_ec);
      }
      local_4 = 0xc;
      if (((bVar3) && (puStack_10 != auStack_90)) && (puStack_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
        scalable_free(puStack_10);
      }
      local_4._0_1_ = 7;
      local_4._1_3_ = 0;
      FUN_0041b420((int *)&local_e8);
      local_4 = CONCAT31(local_4._1_3_,4);
      FMapPackageFileCache__unknown_00d08060(local_d0);
      local_4 = 0xffffffff;
      FMapPackageFileCache__unknown_00d08060(local_dc);
      ExceptionList = local_c;
      return pGVar4;
    }
    FUN_00486000("0",".\\Src\\GFxUIEngine.cpp",0x94);
    pGVar4 = (GRefCountBaseImpl *)FUN_00455b00(0x28);
    if (pGVar4 != (GRefCountBaseImpl *)0x0) {
      *(undefined4 *)(pGVar4 + 4) = 0x56471e89;
      *(undefined4 *)(pGVar4 + 8) = 0x9fe1234a;
    }
    local_4 = 0x12;
  }
  if (pGVar4 == (GRefCountBaseImpl *)0x0) {
    ExceptionList = local_c;
    return (GRefCountBaseImpl *)0x0;
  }
  pGVar4 = FUN_013cf7e0(pGVar4,0,(GRefCountBaseImpl)0x0);
  ExceptionList = local_c;
  return pGVar4;
}




/* ========== FGFxImageInfo.c ========== */

/*
 * SGW.exe - FGFxImageInfo (3 functions)
 */

/* Also in vtable: FGFxImageInfo (slot 0) */

GRefCountBaseImpl * __thiscall FGFxImageInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00af0760(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}


/* [VTable] FGFxImageInfo virtual function #2
   VTable: vtable_FGFxImageInfo at 01960cf4 */

undefined4 __fastcall FGFxImageInfo__vfunc_2(int param_1)

{
  return *(undefined4 *)(param_1 + 0x20);
}


/* [VTable] FGFxImageInfo virtual function #1
   VTable: vtable_FGFxImageInfo at 01960cf4 */

undefined4 __fastcall FGFxImageInfo__vfunc_1(int param_1)

{
  return *(undefined4 *)(param_1 + 0x1c);
}




/* ========== FGFxRenderer.c ========== */

/*
 * SGW.exe - FGFxRenderer (24 functions)
 */

/* [VTable] FGFxRenderer virtual function #1
   VTable: vtable_FGFxRenderer at 019628cc */

undefined4 FGFxRenderer__vfunc_1(undefined4 *param_1)

{
  param_1[2] = 0x36b;
  param_1[1] = 0x1b;
  *param_1 = 0x3304;
  param_1[3] = 0x1000;
  return CONCAT31((int3)((uint)param_1 >> 8),1);
}


/* [VTable] FGFxRenderer virtual function #27
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_27(void *this,undefined8 *param_1,char param_2)

{
  if (param_1 != (undefined8 *)0x0) {
    *param_1 = *(undefined8 *)((int)this + 0x10);
    *(undefined4 *)(param_1 + 1) = *(undefined4 *)((int)this + 0x18);
  }
  if (param_2 != '\0') {
    *(undefined4 *)((int)this + 0x10) = 0;
    *(undefined4 *)((int)this + 0x14) = 0;
    *(undefined4 *)((int)this + 0x18) = 0;
  }
  return;
}


/* [VTable] FGFxRenderer virtual function #19
   VTable: vtable_FGFxRenderer at 019628cc */

void __fastcall FGFxRenderer__vfunc_19(int param_1)

{
  FUN_00af3e00(param_1 + 0xe8);
  return;
}


/* [VTable] FGFxRenderer virtual function #17
   VTable: vtable_FGFxRenderer at 019628cc */

void __fastcall FGFxRenderer__vfunc_17(int param_1)

{
  FUN_00af3e00(param_1 + 0x170);
  return;
}


/* [VTable] FGFxRenderer virtual function #28
   VTable: vtable_FGFxRenderer at 019628cc */

void __fastcall FGFxRenderer__vfunc_28(int param_1)

{
  undefined4 *puVar1;
  void *this;
  
  this = *(void **)(param_1 + 0x9c);
  if (this != (void *)(param_1 + 0x98)) {
    do {
      puVar1 = *(undefined4 **)((int)this + 0xc);
      *puVar1 = 0;
      puVar1[1] = 0;
      FUN_00af6ec0(this,(DWORD *)(param_1 + 0xa0));
      this = *(void **)(param_1 + 0x9c);
    } while (this != (void *)(param_1 + 0x98));
  }
  return;
}


/* [VTable] FGFxRenderer virtual function #2
   VTable: vtable_FGFxRenderer at 019628cc */

undefined4 __fastcall FGFxRenderer__vfunc_2(int param_1)

{
  void *this;
  undefined4 uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016daebb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  if (*(int *)(param_1 + 0x1c) == 0) {
    FUN_00486000("NULL != pGCManager",".\\Src\\GFxUIRenderer.cpp",0x748);
  }
  this = (void *)FUN_00455b00(0x24);
  local_4 = 0;
  if (this == (void *)0x0) {
    uVar1 = 0;
  }
  else {
    uVar1 = FUN_00af6770(this,param_1,*(undefined4 *)(param_1 + 0x1c));
  }
  ExceptionList = local_c;
  return uVar1;
}


/* [VTable] FGFxRenderer virtual function #9
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_9(void *this,undefined8 *param_1)

{
  FUN_00af6490(param_1,(int)this + 0x74);
  return;
}


/* [VTable] FGFxRenderer virtual function #14
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_14(void *this,int param_1)

{
  FUN_00af6ec0(*(void **)(param_1 + 4),(DWORD *)((int)this + 0xa0));
  return;
}


/* [VTable] FGFxRenderer virtual function #7
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_7(void *this,undefined4 *param_1)

{
  FUN_00af94c0(param_1,(int)this + 0x44);
  return;
}


/* [VTable] FGFxRenderer virtual function #8
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_8(void *this,undefined4 *param_1)

{
  FUN_00af94c0(param_1,(int)this + 0x2c);
  return;
}


/* [VTable] FGFxRenderer virtual function #21
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_21(int *param_1)

{
  int iVar1;
  undefined4 local_88;
  int *local_84;
  void *local_80;
  int local_7c [4];
  int local_6c [3];
  GMatrix2D local_60 [32];
  undefined1 local_40 [4];
  int local_3c;
  undefined4 local_38 [11];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db33c;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_6c[0] = 0;
  local_6c[1] = 0;
  local_6c[2] = 0;
  GMatrix2D::SetIdentity(local_60);
  FUN_00af22a0(param_1,local_6c);
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0x9b6);
  }
  if (DAT_01ee2618 == 0) {
    FUN_00af75a0(local_40,&local_88,local_6c);
    local_4 = 2;
    FUN_00af3f10((void *)(local_3c + 0xe8),local_38,(undefined8 *)(local_3c + 0x74));
  }
  else {
    local_84 = FUN_00419650(local_7c,&DAT_01ee2634,0x34);
    local_80 = (void *)local_84[2];
    local_4 = 1;
    if (local_80 != (void *)0x0) {
      FUN_00af75a0(local_80,&local_88,local_6c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_7c);
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #20
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_20(void)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 local_28;
  int *local_24;
  void *local_20;
  int local_1c;
  int local_18;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db3e9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0x9cd);
  }
  if (DAT_01ee2618 != 0) {
    local_24 = FUN_00419650(&local_1c,&DAT_01ee2634,0xc);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      FUN_00af4360(local_20,&local_28,(undefined4 *)&stack0x00000004);
    }
    local_4 = 0xffffffff;
    FUN_00419230(&local_1c);
    ExceptionList = local_c;
    return;
  }
  FUN_00af4360(&local_1c,&local_28,(undefined4 *)&stack0x00000004);
  local_4 = 2;
  puVar2 = FUN_013d3c70((void *)(local_18 + 0x74),&stack0x00000004);
  *(undefined4 *)(local_18 + 0xf0) = *puVar2;
  *(undefined4 *)(local_18 + 0xec) = 1;
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #22
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall
FGFxRenderer__vfunc_22(void *this,undefined4 param_1,int *param_2,int *param_3,int *param_4)

{
  int iVar1;
  int *piVar2;
  int local_b8 [4];
  undefined1 local_a8 [4];
  int local_a4;
  undefined4 local_a0;
  undefined4 *local_9c;
  undefined4 *local_98;
  int local_94;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db398;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0xa26);
  }
  if (DAT_01ee2618 == 0) {
    FUN_00af2a40(local_a8,this,param_1,param_2,param_3,param_4);
    local_4 = 2;
    FUN_00af3f90((void *)(local_a4 + 0xe8),local_a0,local_9c,local_98,local_94,
                 (undefined8 *)(local_a4 + 0x74));
  }
  else {
    piVar2 = FUN_00419650(local_b8,&DAT_01ee2634,0x9c);
    local_4 = 1;
    if ((void *)piVar2[2] != (void *)0x0) {
      FUN_00af2a40((void *)piVar2[2],this,param_1,param_2,param_3,param_4);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_b8);
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #18
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_18(void)

{
  int iVar1;
  undefined4 *puVar2;
  undefined4 local_28;
  int *local_24;
  void *local_20;
  int local_1c;
  int local_18;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db3e9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0xa49);
  }
  if (DAT_01ee2618 != 0) {
    local_24 = FUN_00419650(&local_1c,&DAT_01ee2634,0xc);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      FUN_00af43e0(local_20,&local_28,(undefined4 *)&stack0x00000004);
    }
    local_4 = 0xffffffff;
    FUN_00419230(&local_1c);
    ExceptionList = local_c;
    return;
  }
  FUN_00af43e0(&local_1c,&local_28,(undefined4 *)&stack0x00000004);
  local_4 = 2;
  puVar2 = FUN_013d3c70((void *)(local_18 + 0x74),&stack0x00000004);
  *(undefined4 *)(local_18 + 0x178) = *puVar2;
  *(undefined4 *)(local_18 + 0x174) = 1;
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #24
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_24(void)

{
  int iVar1;
  undefined4 local_28;
  int *local_24;
  void *local_20;
  int local_1c;
  int local_18;
  void *local_14;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db3e9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0xa69);
  }
  if (DAT_01ee2618 != 0) {
    local_24 = FUN_00419650(&local_1c,&DAT_01ee2634,0xc);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      FUN_00af2b80(local_20,(undefined4 *)&stack0x00000004,&local_28);
    }
    local_4 = 0xffffffff;
    FUN_00419230(&local_1c);
    ExceptionList = local_c;
    return;
  }
  FUN_00af2b80(&local_1c,(undefined4 *)&stack0x00000004,&local_28);
  local_4 = 2;
  FUN_00af4460(local_14,local_18);
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #25
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_25(void)

{
  int iVar1;
  undefined4 local_2c;
  int *local_28;
  void *local_24;
  int local_20;
  int local_1c [4];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db7f9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0xac9);
  }
  if (DAT_01ee2618 != 0) {
    local_28 = FUN_00419650(local_1c,&DAT_01ee2634,8);
    local_24 = (void *)local_28[2];
    local_4 = 1;
    if (local_24 != (void *)0x0) {
      FUN_00af2c50(local_24,&local_2c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_1c);
    ExceptionList = local_c;
    return;
  }
  FUN_00af2c50(&local_24,&local_2c);
  local_4 = 2;
  FUN_00af45e0(local_20);
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #11
   VTable: vtable_FGFxRenderer at 019628cc */

void __fastcall FGFxRenderer__vfunc_11(int param_1)

{
  int iVar1;
  int local_2c;
  int local_28;
  int *local_24;
  void *local_20;
  int local_1c [4];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db6d9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0x823);
  }
  if (DAT_01ee2618 != 0) {
    local_24 = FUN_00419650(local_1c,&DAT_01ee2634,0xc);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      local_2c = param_1 + 0xb8;
      local_28 = param_1 + 0xb4;
      FUN_00af2880(local_20,&local_28,&local_2c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_1c);
    ExceptionList = local_c;
    return;
  }
  local_28 = param_1 + 0xb8;
  local_2c = param_1 + 0xb4;
  FUN_00af2880(local_1c,&local_2c,&local_28);
  local_4 = 2;
  FUN_00af9c30((int)local_1c);
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #6
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_6(void)

{
  int iVar1;
  undefined4 local_2c;
  int *local_28;
  void *local_24;
  int local_20;
  int local_1c [4];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db7f9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0x7d3);
  }
  if (DAT_01ee2618 != 0) {
    local_28 = FUN_00419650(local_1c,&DAT_01ee2634,8);
    local_24 = (void *)local_28[2];
    local_4 = 1;
    if (local_24 != (void *)0x0) {
      FUN_00af2790(local_24,&local_2c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_1c);
    ExceptionList = local_c;
    return;
  }
  FUN_00af2790(&local_24,&local_2c);
  local_4 = 2;
  FUN_00af9b70(local_20);
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #10
   VTable: vtable_FGFxRenderer at 019628cc */

void __fastcall FGFxRenderer__vfunc_10(int param_1)

{
  int iVar1;
  int *piVar2;
  int local_2c;
  int local_28;
  int *local_24;
  void *local_20;
  int local_1c;
  int local_18;
  int *local_14;
  int *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db6d9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0x80c);
  }
  if (DAT_01ee2618 != 0) {
    local_24 = FUN_00419650(&local_1c,&DAT_01ee2634,0x10);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      local_2c = param_1 + 0xb8;
      local_28 = param_1 + 0xb4;
      FUN_00af2800(local_20,(undefined4 *)&stack0x00000004,&local_28,&local_2c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(&local_1c);
    ExceptionList = local_c;
    return;
  }
  local_28 = param_1 + 0xb8;
  local_2c = param_1 + 0xb4;
  FUN_00af2800(&local_1c,(undefined4 *)&stack0x00000004,&local_2c,&local_28);
  local_4 = 2;
  piVar2 = (int *)FUN_00af6650(4,local_10);
  if (piVar2 != (int *)0x0) {
    *piVar2 = *local_14;
  }
  if ((2 < local_18) && (*local_14 != local_18)) {
    *local_14 = local_18;
    FUN_00af9660(local_18);
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #26
   VTable: vtable_FGFxRenderer at 019628cc */

void FGFxRenderer__vfunc_26(void)

{
  int iVar1;
  undefined4 local_2c;
  int *local_28;
  void *local_24;
  undefined4 *local_20;
  int local_1c [4];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db7f9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0xaf9);
  }
  if (DAT_01ee2618 != 0) {
    local_28 = FUN_00419650(local_1c,&DAT_01ee2634,8);
    local_24 = (void *)local_28[2];
    local_4 = 1;
    if (local_24 != (void *)0x0) {
      FUN_00af2cc0(local_24,&local_2c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_1c);
    ExceptionList = local_c;
    return;
  }
  FUN_00af2cc0(&local_24,&local_2c);
  local_4 = 2;
  FUN_00afa380(local_20);
  ExceptionList = local_c;
  return;
}


/* Also in vtable: FGFxRenderer (slot 0) */

GRefCountBaseImpl * __thiscall FGFxRenderer__vfunc_0(void *this,byte param_1)

{
  FUN_00afdbd0(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}


/* [VTable] FGFxRenderer virtual function #16
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_16(void *this,undefined4 param_1,int param_2)

{
  int iVar1;
  void *local_28;
  int *local_24;
  void *local_20;
  int local_1c;
  int local_18;
  int local_14;
  void *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db3e9;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(int *)((int)this + 0x14) = *(int *)((int)this + 0x14) + param_2;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0x8b9);
  }
  if (DAT_01ee2618 != 0) {
    local_24 = FUN_00419650(&local_1c,&DAT_01ee2634,0x10);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      local_28 = this;
      FUN_00af2990(local_20,&param_1,&param_2,&local_28);
    }
    local_4 = 0xffffffff;
    FUN_00419230(&local_1c);
    ExceptionList = local_c;
    return;
  }
  local_28 = this;
  FUN_00af2990(&local_1c,&param_1,&param_2,&local_28);
  local_4 = 2;
  FUN_00afd1a0(local_10,local_18,local_14);
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxRenderer virtual function #12
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall
FGFxRenderer__vfunc_12(void *this,int param_1,int param_2,undefined4 param_3,int *param_4)

{
  FUN_00afead0((int)this,1,param_1,param_2,param_3,param_4,(int)this + 0x24);
  return;
}


/* [VTable] FGFxRenderer virtual function #13
   VTable: vtable_FGFxRenderer at 019628cc */

void __thiscall FGFxRenderer__vfunc_13(void *this,int param_1,int param_2,int param_3,int *param_4)

{
  FUN_00afee30((int)this,2,param_1,param_2,param_3,param_4,(int)this + 0x28);
  return;
}




/* ========== FGFxRenderer_FGFxFillStyle.c ========== */

/*
 * SGW.exe - FGFxRenderer_FGFxFillStyle (3 functions)
 */

/* [VTable] FGFxRenderer_FGFxFillStyle virtual function #2
   VTable: vtable_FGFxRenderer_FGFxFillStyle at 01961928 */

void __thiscall FGFxRenderer_FGFxFillStyle__vfunc_2(void *this,int *param_1,int *param_2)

{
  if (*(int *)((int)this + 4) == 0) {
    FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x5fd);
    if (*(int *)((int)this + 4) == 0) {
      FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x5bc);
    }
  }
  if (*param_2 == 1) {
    param_1[1] = 0x1000;
    param_1[2] = 0x80000;
  }
  *param_1 = 1;
  if (*(int *)((int)this + 4) == 2) {
    if ((param_2[1] != 3) && (param_2[1] != 6)) {
      *param_1 = 2;
      return;
    }
    *param_1 = 4;
    return;
  }
  if (*(int *)((int)this + 4) == 3) {
    if (*param_2 == 3) {
      param_1[1] = 0x4000;
      param_1[2] = 0x200000;
    }
    else if (*param_2 == 4) {
      param_1[2] = 0x400000;
      if (*(int *)((int)this + 0x2c) == 0) {
        param_1[1] = 0x10000;
      }
      else {
        param_1[1] = (-(uint)(*(int *)((int)this + 0x2c) != 3) & 0xfffc8000) + 0x40000;
      }
    }
    else {
      FUN_00486000("0",".\\Src\\GFxUIRenderer.cpp",0x632);
    }
    *param_1 = 0;
    if (*(int *)((int)this + 0x30) == 0) {
      if (*param_2 == 3) {
        param_1[1] = 0x20000;
        *param_1 = 0x20;
      }
      else {
        *param_1 = 0x10;
      }
    }
    else if ((*(int *)((int)this + 0x2c) == 2) || (*(int *)((int)this + 0x2c) == 1)) {
      *param_1 = 0x40;
    }
    else {
      *param_1 = 0x80;
    }
    if (((param_2[1] == 3) || (param_2[1] == 6)) && (*param_1 = *param_1 << 4, 0xb < *param_1)) {
      FUN_00486000("EnumeratedBoundShaderState.PixelShaderType <= GFx_PS_LastBit",
                   ".\\Src\\GFxUIRenderer.cpp",0x659);
    }
  }
  return;
}


/* Also in vtable: FGFxRenderer_FGFxFillStyle (slot 0) */

void __fastcall FGFxRenderer_FGFxFillStyle__vfunc_0(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 8) = 0xff0000;
  *(undefined8 *)(param_1 + 0xc) = DAT_01f11a74;
  *(undefined8 *)(param_1 + 0x14) = DAT_01f11a7c;
  *(undefined8 *)(param_1 + 0x1c) = DAT_01f11a84;
  *(undefined8 *)(param_1 + 0x24) = DAT_01f11a8c;
  *(undefined4 *)(param_1 + 0x2c) = 0;
  *(undefined4 *)(param_1 + 0x30) = 0;
  *(undefined4 *)(param_1 + 0x34) = 0;
  *(undefined4 *)(param_1 + 0x38) = 0;
  *(undefined4 *)(param_1 + 0x3c) = DAT_01f11a5c;
  *(undefined4 *)(param_1 + 0x40) = DAT_01f11a60;
  *(undefined4 *)(param_1 + 0x44) = DAT_01f11a64;
  *(undefined4 *)(param_1 + 0x48) = DAT_01f11a68;
  *(undefined4 *)(param_1 + 0x4c) = DAT_01f11a6c;
  *(undefined4 *)(param_1 + 0x50) = DAT_01f11a70;
  *(undefined4 *)(param_1 + 0x54) = 1;
  *(undefined4 *)(param_1 + 0x58) = 1;
  *(undefined4 *)(param_1 + 0x5c) = 0;
  *(undefined4 *)(param_1 + 0x60) = 0;
  *(undefined4 *)(param_1 + 100) = 0;
  *(undefined4 *)(param_1 + 0x68) = DAT_01f11a5c;
  *(undefined4 *)(param_1 + 0x6c) = DAT_01f11a60;
  *(undefined4 *)(param_1 + 0x70) = DAT_01f11a64;
  *(undefined4 *)(param_1 + 0x74) = DAT_01f11a68;
  *(undefined4 *)(param_1 + 0x78) = DAT_01f11a6c;
  *(undefined4 *)(param_1 + 0x7c) = DAT_01f11a70;
  *(undefined4 *)(param_1 + 0x80) = 1;
  *(undefined4 *)(param_1 + 0x84) = 1;
  return;
}


/* [VTable] FGFxRenderer_FGFxFillStyle virtual function #1
   VTable: vtable_FGFxRenderer_FGFxFillStyle at 01961928 */

void __thiscall
FGFxRenderer_FGFxFillStyle__vfunc_1(void *this,void *param_1,undefined4 *param_2,int param_3)

{
  int *piVar1;
  
  if (*(int *)((int)this + 4) == 0) {
    FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x661);
  }
  FGFxRenderer_FGFxRenderStyle__vfunc_1(this,param_1,param_2,param_3);
  if (*(int *)((int)this + 4) != 1) {
    *(undefined4 *)(param_3 + 8) = 0;
    FUN_00af4060((int)param_2,param_3,(int *)((int)this + 0x30));
    FUN_00af71b0(param_1,param_2,param_3,(int *)((int)this + 0x30));
    piVar1 = (int *)((int)this + 0x5c);
    if (((*piVar1 != 0) && (*(int *)((int)this + 0x2c) == 4)) || (*(int *)((int)this + 0x2c) == 3))
    {
      *(undefined4 *)(param_3 + 8) = 1;
      FUN_00af4060((int)param_2,param_3,piVar1);
      FUN_00af71b0(param_1,param_2,param_3,piVar1);
    }
  }
  return;
}




/* ========== FGFxTexture.c ========== */

/*
 * SGW.exe - FGFxTexture (6 functions)
 */

/* [VTable] FGFxTexture virtual function #11
   VTable: vtable_FGFxTexture at 01962170 */

undefined4 __thiscall FGFxTexture__vfunc_11(void *this,int param_1,int param_2,int param_3)

{
  uint *puVar1;
  bool bVar2;
  int iVar3;
  undefined3 extraout_var;
  undefined4 uVar4;
  undefined4 extraout_ECX;
  undefined4 extraout_ECX_00;
  undefined4 extraout_EDX;
  undefined4 extraout_EDX_00;
  ulonglong uVar5;
  
  if (*(int *)((int)this + 0x10) != 0) {
    FGFxTexture__unknown_00af4710((int)this);
  }
  *(int *)((int)this + 0x10) = param_1;
  iVar3 = FUN_006cab50(param_1);
  *(int *)((int)this + 0x14) = iVar3;
  iVar3 = FUN_00a31ed0(*(int *)((int)this + 0x10));
  *(int *)((int)this + 0x18) = iVar3;
  if (param_2 == 0) {
    (**(code **)(**(int **)((int)this + 0x10) + 0x10c))();
    uVar5 = CEGUI__unknown_012379c0(extraout_ECX,extraout_EDX);
    param_2 = (int)uVar5;
  }
  *(int *)((int)this + 0x1c) = param_2;
  if (param_3 == 0) {
    (**(code **)(**(int **)((int)this + 0x10) + 0x110))();
    uVar5 = CEGUI__unknown_012379c0(extraout_ECX_00,extraout_EDX_00);
    param_3 = (int)uVar5;
  }
  *(int *)((int)this + 0x20) = param_3;
  if (*(int *)((int)this + 0x10) == 0) {
    FUN_00486000("NULL != pTexture",".\\Src\\GFxUIRenderer.cpp",0xb09);
  }
  bVar2 = FUN_00af1dc0(*(void **)((int)this + 0xc),*(int *)((int)this + 0x10));
  uVar4 = 0;
  if (CONCAT31(extraout_var,bVar2) != 1) {
    uVar4 = FUN_00486000("TRUE == Result",".\\Src\\GFxUIRenderer.cpp",0xb0c);
  }
  puVar1 = (uint *)(*(int *)((int)this + 0x14) + 0x3c);
  *puVar1 = *puVar1 & 0xfffffffe;
  return CONCAT31((int3)((uint)uVar4 >> 8),1);
}


/* [VTable] FGFxTexture virtual function #6
   VTable: vtable_FGFxTexture at 01962170 */

undefined4 __fastcall FGFxTexture__vfunc_6(int param_1)

{
  if ((*(int *)(param_1 + 0x1c) != 0) && (*(int *)(param_1 + 0x20) != 0)) {
    return 1;
  }
  return 0;
}


/* [VTable] FGFxTexture virtual function #3
   VTable: vtable_FGFxTexture at 01962170 */

uint __thiscall FGFxTexture__vfunc_3(void *this,LPCSTR param_1,int param_2,int param_3)

{
  undefined **ppuVar1;
  uint uVar2;
  LPWSTR pWVar3;
  uint uVar4;
  undefined4 uVar5;
  undefined1 local_110 [256];
  undefined1 *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016dad0b;
  local_c = ExceptionList;
  uVar2 = DAT_01e7f9a0 ^ (uint)&stack0xfffffee8;
  ExceptionList = &local_c;
  pWVar3 = CEGUI__unknown_00423e50(local_110,param_1);
  local_4 = 0;
  ppuVar1 = *(undefined ***)(pWVar3 + 0x80);
  if (ppuVar1 == (undefined **)0x0) {
    uVar4 = 0;
  }
  else {
    if (DAT_01ee6588 == (undefined4 *)0x0) {
      DAT_01ee6588 = WxUnrealEdApp__unknown_008df850();
      WxUnrealEdApp__unknown_008dd840();
    }
    uVar4 = FUN_004a8e10(DAT_01ee6588,(undefined4 *)0x0,ppuVar1,(wchar_t *)0x0,0,(int *)0x0,1);
  }
  *(uint *)((int)this + 0x10) = uVar4;
  local_4 = 0xffffffff;
  if ((local_10 != local_110) && (local_10 != (undefined1 *)0x0)) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_10,uVar2);
  }
  if (*(int *)((int)this + 0x10) == 0) {
    uVar2 = (uint)local_10 & 0xffffff00;
  }
  else {
    if ((param_2 == 0) || (param_3 == 0)) {
      *(undefined4 *)((int)this + 0x1c) = *(undefined4 *)(*(int *)((int)this + 0x14) + 200);
      *(undefined4 *)((int)this + 0x20) = *(undefined4 *)(*(int *)((int)this + 0x14) + 0xcc);
    }
    else {
      *(int *)((int)this + 0x1c) = param_2;
      *(int *)((int)this + 0x20) = param_3;
    }
    uVar5 = FUN_00af46c0((int)this);
    uVar2 = CONCAT31((int3)((uint)uVar5 >> 8),1);
  }
  ExceptionList = local_c;
  return uVar2;
}


/* Also in vtable: FGFxTexture (slot 0) */

undefined4 * __thiscall FGFxTexture__vfunc_0(void *this,byte param_1)

{
  FUN_00af6820(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}


/* [VTable] FGFxTexture virtual function #2
   VTable: vtable_FGFxTexture at 01962170 */

undefined4 __thiscall FGFxTexture__vfunc_2(void *this,int *param_1,void *param_2,uint param_3)

{
  uint *puVar1;
  int *this_00;
  undefined4 in_EAX;
  void *pvVar2;
  undefined4 *puVar3;
  undefined1 *puVar4;
  uint uVar5;
  uint uVar6;
  int iVar7;
  int iVar8;
  undefined *puVar9;
  undefined *puVar10;
  uint local_1c;
  int local_18;
  void *local_14;
  uint local_10;
  size_t local_c;
  void *local_8;
  void *local_4;
  
  this_00 = param_1;
  local_4 = this;
  if (*(int *)((int)this + 0x10) != 0) {
    in_EAX = FGFxTexture__unknown_00af4710((int)this);
  }
  if (param_1 != (int *)0x0) {
    iVar7 = *param_1;
    uVar5 = 1;
    param_1 = (int *)0x0;
    local_18 = 1;
    local_1c = 2;
    if (iVar7 == 1) {
      param_1 = (int *)&DAT_00000004;
      local_18 = 4;
      local_1c = 2;
    }
    else if (iVar7 == 2) {
      param_1 = (int *)&DAT_00000003;
      local_18 = 4;
      local_1c = 2;
    }
    else if (iVar7 == 9) {
      param_1 = (int *)0x1;
      local_1c = 3;
    }
    else if (iVar7 == 10) {
      param_1 = (int *)0x1;
      local_1c = 5;
    }
    else if (iVar7 == 0xb) {
      param_1 = (int *)0x1;
      local_1c = 6;
    }
    else if (iVar7 == 0xc) {
      param_1 = (int *)0x1;
      local_1c = 7;
    }
    else {
      FUN_00486000("0",".\\Src\\GFxUIRenderer.cpp",3000);
    }
    pvVar2 = param_2;
    if (param_2 == (void *)0x0) {
      pvVar2 = (void *)this_00[1];
    }
    *(void **)((int)this + 0x1c) = pvVar2;
    uVar6 = param_3;
    if (param_3 == 0) {
      uVar6 = this_00[2];
    }
    *(uint *)((int)this + 0x20) = uVar6;
    if ((*this_00 < 10) || (0xc < *this_00)) {
      uVar6 = 1;
      if (1 < (uint)this_00[1]) {
        do {
          uVar6 = uVar6 * 2;
        } while (uVar6 < (uint)this_00[1]);
      }
      if (1 < (uint)this_00[2]) {
        do {
          uVar5 = uVar5 * 2;
        } while (uVar5 < (uint)this_00[2]);
      }
    }
    else {
      uVar6 = this_00[1];
      uVar5 = this_00[2];
    }
    puVar10 = (undefined *)0x0;
    puVar9 = (undefined *)0x0;
    iVar7 = 0;
    iVar8 = 0;
    pvVar2 = (void *)FUN_0049ed80();
    puVar3 = (undefined4 *)FUN_00af1ff0(0xf4,pvVar2,iVar7,iVar8,puVar9,puVar10);
    if (puVar3 == (undefined4 *)0x0) {
      puVar3 = (undefined4 *)0x0;
    }
    else {
      puVar3 = FUN_0075cff0(puVar3);
    }
    *(undefined4 **)((int)this + 0x14) = puVar3;
    *(undefined4 **)((int)this + 0x10) = puVar3;
    FUN_00af46c0((int)this);
    *(undefined4 *)(*(int *)((int)this + 0x14) + 0xd8) = 1;
    FUN_008de4a0(*(void **)((int)this + 0x14),uVar6,uVar5,local_1c,0);
    puVar1 = (uint *)(*(int *)((int)this + 0x14) + 0x3c);
    *puVar1 = *puVar1 & 0xfffffffe;
    if (*(int *)(*(int *)((int)this + 0x14) + 0xd8) != 1) {
      FUN_00486000("pTexture2D->RequestedMips == 1",".\\Src\\GFxUIRenderer.cpp",0xbd4);
    }
    if ((uVar6 == this_00[1]) && (uVar5 == this_00[2])) {
      if (((uint)this_00[6] < 2) && ((*this_00 < 10 || (0xc < *this_00)))) {
        FUN_00af4770(this,0,(void *)(uVar6 * local_18),(void *)this_00[4],uVar6,uVar5,param_1,
                     this_00[3]);
      }
      else {
        uVar5 = 0;
        do {
          local_8 = (void *)FUN_013c7970(this_00,uVar5,&local_1c,&param_3,(uint *)&param_2);
          if (local_8 == (void *)0x0) break;
          local_14 = param_2;
          local_10 = param_3;
          local_c = local_1c;
          if (*(int *)((int)this + 0x14) != 0) {
            pvVar2 = *(void **)(*(int *)(*(int *)((int)this + 0x14) + 0xbc) + uVar5 * 4);
            puVar4 = (undefined1 *)FUN_005072a0(pvVar2,2);
            FUN_00af2e50((void *)(uVar6 * local_18),puVar4,local_8,local_c,local_10,(int)local_14);
            FUN_00506ad0((int)pvVar2);
            this = local_4;
          }
          uVar5 = uVar5 + 1;
        } while (uVar5 < (uint)this_00[6]);
      }
    }
    else {
      param_2 = (void *)FUN_00af2d30(*this_00,this_00[1],this_00[2],this_00[3],this_00[4],uVar5);
      if (*(int *)((int)this + 0x14) != 0) {
        pvVar2 = (void *)**(undefined4 **)(*(int *)((int)this + 0x14) + 0xbc);
        puVar4 = (undefined1 *)FUN_005072a0(pvVar2,2);
        FUN_00af2e50((void *)(uVar6 * local_18),puVar4,param_2,uVar6,uVar5,uVar6 * (int)param_1);
        FUN_00506ad0((int)pvVar2);
      }
      FUN_00455b50(param_2);
    }
    in_EAX = (**(code **)(**(int **)((int)this + 0x14) + 0x114))();
  }
  return CONCAT31((int3)((uint)in_EAX >> 8),1);
}


/* [VTable] FGFxTexture virtual function #4
   VTable: vtable_FGFxTexture at 01962170 */

void __thiscall
FGFxTexture__vfunc_4(void *this,int param_1,int param_2,undefined4 param_3,GImageBase *param_4)

{
  uint uVar1;
  int iVar2;
  void *pvVar3;
  int *piVar4;
  undefined4 *puVar5;
  int *piVar6;
  int iVar7;
  void *unaff_EBP;
  int unaff_retaddr;
  void *local_4c;
  GImage *local_48;
  undefined4 local_44;
  int local_40;
  int local_3c;
  int local_38;
  undefined **ppuStack_2c;
  undefined8 uStack_28;
  undefined8 uStack_20;
  void *pvStack_18;
  void *pvStack_14;
  void *local_c;
  undefined1 *puStack_8;
  int *local_4;
  
  local_4 = (int *)0xffffffff;
  puStack_8 = &LAB_016dbb4c;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffa4;
  ExceptionList = &local_c;
  local_44 = 0;
  local_40 = 0;
  local_3c = param_1;
  local_38 = param_2;
  local_4c = this;
  local_48 = (GImage *)FUN_00455b00(0x38);
  if (local_48 != (GImage *)0x0) {
    *(undefined4 *)(local_48 + 4) = 0x56471e89;
    *(undefined4 *)(local_48 + 8) = 0x9fe1234a;
  }
  local_4 = (int *)0x0;
  if (local_48 == (GImage *)0x0) {
    iVar2 = 0;
  }
  else {
    iVar2 = GImage::GImage(local_48,param_4);
  }
  local_4 = (int *)0xffffffff;
  local_40 = iVar2;
  if (DAT_01ea5778 == (int *)0x0) {
    FUN_00416660();
  }
  pvVar3 = (void *)(**(code **)(*DAT_01ea5778 + 4))(param_2 * 0x18,8,uVar1);
  if (0 < param_2) {
    piVar6 = (int *)((int)pvVar3 + 8);
    piVar4 = (int *)(param_1 + 0x14);
    iVar7 = param_2;
    do {
      piVar6[-2] = piVar4[-5];
      piVar6[-1] = piVar4[-4];
      *piVar6 = piVar4[-3];
      piVar6[1] = piVar4[-2];
      piVar6[2] = piVar4[-1] - piVar4[-3];
      piVar6 = piVar6 + 6;
      iVar7 = iVar7 + -1;
      *(int *)((int)pvVar3 + (-0x18 - param_1) + (int)(piVar4 + 6)) = *piVar4 - piVar4[-2];
      piVar4 = piVar4 + 6;
      iVar2 = unaff_retaddr;
      this = unaff_EBP;
    } while (iVar7 != 0);
  }
  local_4c = pvVar3;
  iVar7 = FUN_0054bb60();
  if (iVar7 == 0) {
    FUN_00486000("IsInGameThread()",".\\Src\\GFxUIRenderer.cpp",0xc91);
  }
  if (DAT_01ee2618 == 0) {
    uStack_28 = CONCAT44(local_48,local_4c);
    uStack_20 = CONCAT44(local_40,local_44);
    ppuStack_2c = `public:_virtual_void___thiscall_FGFxTexture::Update(int,int,struct_GTexture::UpdateRect_const*,class_GImageBase_const*)'
                  ::__l5::UpdateCommand::vftable;
    local_c = (void *)0x4;
    if (iVar2 == 0) {
      puVar5 = (undefined4 *)0x0;
    }
    else {
      puVar5 = (undefined4 *)(iVar2 + 0x10);
    }
    pvStack_18 = this;
    FUN_00af7750(this,local_4,param_2,(int)pvVar3,puVar5);
    if (DAT_01ea5778 == (int *)0x0) {
      FUN_00416660();
    }
    (**(code **)(*DAT_01ea5778 + 0xc))(pvVar3);
    (**(code **)(*(int *)(iVar2 + 8) + 4))();
  }
  else {
    local_4 = FUN_00419650(&local_3c,&DAT_01ee2634,0x18);
    local_c = (void *)0x2;
    if ((void *)local_4[2] != (void *)0x0) {
      FUN_00af2f80((void *)local_4[2],(undefined8 *)&local_4c,(undefined4 *)&stack0x00000000);
    }
    local_c = (void *)0xffffffff;
    FUN_00419230(&local_3c);
  }
  ExceptionList = pvStack_14;
  return;
}




/* ========== FGFxUpdatableTexture.c ========== */

/*
 * SGW.exe - FGFxUpdatableTexture (4 functions)
 */

/* [VTable] FGFxUpdatableTexture virtual function #8
   VTable: vtable_FGFxUpdatableTexture at 0196239c */

undefined4 __fastcall FGFxUpdatableTexture__vfunc_8(int param_1)

{
  return *(undefined4 *)(param_1 + 0x30);
}


/* [VTable] FGFxUpdatableTexture virtual function #3
   VTable: vtable_FGFxUpdatableTexture at 0196239c */

void __fastcall FGFxUpdatableTexture__vfunc_3(int param_1)

{
  int *piVar1;
  undefined4 *puVar2;
  
  piVar1 = *(int **)(param_1 + 0x14);
  *(undefined4 *)(param_1 + 0x14) = 0;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  puVar2 = *(undefined4 **)(param_1 + 0x1c);
  *(undefined4 *)(param_1 + 0x1c) = 0;
  if (puVar2 != (undefined4 *)0x0) {
    piVar1 = puVar2 + 1;
    *piVar1 = *piVar1 + -1;
    if (*piVar1 == 0) {
      (**(code **)*puVar2)(1);
    }
  }
  piVar1 = *(int **)(param_1 + 0x3c);
  *(undefined4 *)(param_1 + 0x3c) = 0;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  return;
}


/* [VTable] FGFxUpdatableTexture virtual function #2
   VTable: vtable_FGFxUpdatableTexture at 0196239c */

void __fastcall FGFxUpdatableTexture__vfunc_2(int param_1)

{
  int iVar1;
  undefined4 *puVar2;
  uint uVar3;
  int *piVar4;
  int *piVar5;
  int *local_28;
  uint local_24;
  int local_20 [5];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016db5c8;
  local_c = ExceptionList;
  uVar3 = DAT_01e7f9a0 ^ (uint)&stack0xffffffcc;
  ExceptionList = &local_c;
  piVar4 = FSceneRenderTargets__unknown_00ec8070
                     ((int *)&local_28,*(LPCSTR *)(param_1 + 0x2c),*(undefined4 *)(param_1 + 0x30),
                      (void *)(uint)*(byte *)(param_1 + 0x38),*(undefined4 *)(param_1 + 0x34),0x10);
  local_4 = 0;
  piVar5 = *(int **)(param_1 + 0x3c);
  *(uint *)(param_1 + 0x40) =
       *(uint *)(param_1 + 0x40) ^ (piVar4[1] ^ *(uint *)(param_1 + 0x40)) & 1;
  piVar4 = (int *)*piVar4;
  *(int *)(param_1 + 0x3c) = (int)piVar4;
  if (piVar4 != (int *)0x0) {
    (**(code **)(*piVar4 + 4))(piVar4,uVar3);
  }
  if (piVar5 != (int *)0x0) {
    (**(code **)(*piVar5 + 8))(piVar5);
  }
  local_4 = 0xffffffff;
  if (local_28 != (int *)0x0) {
    (**(code **)(*local_28 + 8))(local_28);
  }
  FUN_005f3a60((int *)(param_1 + 0x3c),&local_28);
  local_4 = 2;
  piVar5 = *(int **)(param_1 + 0x14);
  *(uint *)(param_1 + 0x18) = *(uint *)(param_1 + 0x18) ^ (*(uint *)(param_1 + 0x18) ^ local_24) & 1
  ;
  *(int **)(param_1 + 0x14) = local_28;
  if (local_28 != (int *)0x0) {
    (**(code **)(*local_28 + 4))(local_28);
  }
  if (piVar5 != (int *)0x0) {
    (**(code **)(*piVar5 + 8))(piVar5);
  }
  local_4 = 0xffffffff;
  if (local_28 != (int *)0x0) {
    (**(code **)(*local_28 + 8))(local_28);
  }
  local_20[0] = 1;
  local_20[1] = 1;
  local_20[2] = 0;
  local_20[3] = 0;
  local_20[4] = 0;
  piVar5 = FUN_00eca110(&local_28,local_20);
  local_4 = 4;
  iVar1 = *piVar5;
  puVar2 = *(undefined4 **)(param_1 + 0x1c);
  *(int *)(param_1 + 0x1c) = iVar1;
  if (iVar1 != 0) {
    *(int *)(iVar1 + 4) = *(int *)(iVar1 + 4) + 1;
  }
  if (puVar2 != (undefined4 *)0x0) {
    piVar5 = puVar2 + 1;
    *piVar5 = *piVar5 + -1;
    if (*piVar5 == 0) {
      (**(code **)*puVar2)(1);
    }
  }
  local_4 = 0xffffffff;
  if ((local_28 != (int *)0x0) && (local_28[1] = local_28[1] + -1, local_28[1] == 0)) {
    (**(code **)*local_28)(1);
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] FGFxUpdatableTexture virtual function #9
   VTable: vtable_FGFxUpdatableTexture at 0196239c */

undefined4 * __thiscall FGFxUpdatableTexture__vfunc_9(void *this,byte param_1)

{
  FUN_00afc3c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASActionBuffer.c ========== */

/*
 * SGW.exe - GASActionBuffer (1 functions)
 */

XImage * __thiscall GASActionBuffer__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XImage::~XImage(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASActionBufferData.c ========== */

/*
 * SGW.exe - GASActionBufferData (1 functions)
 */

XQAT * __thiscall GASActionBufferData__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XQAT::~XQAT(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASArrayCtorFunction.c ========== */

/*
 * SGW.exe - GASArrayCtorFunction (1 functions)
 */

GRefCountBaseImpl * __thiscall GASArrayCtorFunction__vfunc_0(void *this,uint param_1)

{
  FUN_01469e60(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASArrayObject.c ========== */

/*
 * SGW.exe - GASArrayObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASArrayObject__vfunc_0(void *this,uint param_1)

{
  FUN_014bd3d0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASArrayProto.c ========== */

/*
 * SGW.exe - GASArrayProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASArrayProto__vfunc_0(void *this,uint param_1)

{
  FUN_014bfc50(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASAsBroadcaster.c ========== */

/*
 * SGW.exe - GASAsBroadcaster (1 functions)
 */

GRefCountBaseImpl * __thiscall GASAsBroadcaster__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASAsBroadcasterProto.c ========== */

/*
 * SGW.exe - GASAsBroadcasterProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASAsBroadcasterProto__vfunc_0(void *this,uint param_1)

{
  FUN_0149ca50(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASAsFunctionDef.c ========== */

/*
 * SGW.exe - GASAsFunctionDef (1 functions)
 */

undefined4 * __thiscall GASAsFunctionDef__vfunc_0(void *this,uint param_1)

{
  FUN_0146ff70(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASBitmapData.c ========== */

/*
 * SGW.exe - GASBitmapData (1 functions)
 */

GRefCountBaseImpl * __thiscall GASBitmapData__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASBitmapDataProto.c ========== */

/*
 * SGW.exe - GASBitmapDataProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASBitmapDataProto__vfunc_0(void *this,uint param_1)

{
  FUN_014ac950(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASBooleanObject.c ========== */

/*
 * SGW.exe - GASBooleanObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASBooleanObject__vfunc_0(void *this,uint param_1)

{
  FUN_014c6d30(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASBooleanProto.c ========== */

/*
 * SGW.exe - GASBooleanProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASBooleanProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d8d20(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASButtonObject.c ========== */

/*
 * SGW.exe - GASButtonObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASButtonObject__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASButtonProto.c ========== */

/*
 * SGW.exe - GASButtonProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASButtonProto__vfunc_0(void *this,uint param_1)

{
  FUN_01466470(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASCFunctionDef.c ========== */

/*
 * SGW.exe - GASCFunctionDef (1 functions)
 */

undefined4 * __thiscall GASCFunctionDef__vfunc_0(void *this,uint param_1)

{
  FUN_0146fab0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASColorProto.c ========== */

/*
 * SGW.exe - GASColorProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASColorProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d7d40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASColorTransformProto.c ========== */

/*
 * SGW.exe - GASColorTransformProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASColorTransformProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d03d0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASDateProto.c ========== */

/*
 * SGW.exe - GASDateProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASDateProto__vfunc_0(void *this,uint param_1)

{
  FUN_014cd260(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASDoAction.c ========== */

/*
 * SGW.exe - GASDoAction (1 functions)
 */

undefined4 * __thiscall GASDoAction__vfunc_0(void *this,uint param_1)

{
  FID_conflict__CChevronOwnerDrawMenu(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASDoInitAction.c ========== */

/*
 * SGW.exe - GASDoInitAction (1 functions)
 */

undefined4 * __thiscall GASDoInitAction__vfunc_0(void *this,uint param_1)

{
  FUN_0142cb10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASEnvironment.c ========== */

/*
 * SGW.exe - GASEnvironment (1 functions)
 */

undefined4 * __thiscall GASEnvironment__vfunc_0(void *this,uint param_1)

{
  FUN_014574b0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASExecuteTag.c ========== */

/*
 * SGW.exe - GASExecuteTag (1 functions)
 */

undefined4 * __thiscall GASExecuteTag__vfunc_0(void *this,uint param_1)

{
  FUN_013df230(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASExternalInterfaceProto.c ========== */

/*
 * SGW.exe - GASExternalInterfaceProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASExternalInterfaceProto__vfunc_0(void *this,uint param_1)

{
  FUN_014ca270(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASFnCall.c ========== */

/*
 * SGW.exe - GASFnCall (1 functions)
 */

undefined4 * __thiscall GASFnCall__vfunc_0(void *this,uint param_1)

{
  FID_conflict__CMFCCmdUsageCount(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASFunctionDef.c ========== */

/*
 * SGW.exe - GASFunctionDef (1 functions)
 */

undefined4 * __thiscall GASFunctionDef__vfunc_0(void *this,uint param_1)

{
  FUN_0146fa90(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASFunctionObject.c ========== */

/*
 * SGW.exe - GASFunctionObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASFunctionObject__vfunc_0(void *this,uint param_1)

{
  FUN_01471550(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASFunctionProto.c ========== */

/*
 * SGW.exe - GASFunctionProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASFunctionProto__vfunc_0(void *this,uint param_1)

{
  FUN_01471e90(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASGlobalContext.c ========== */

/*
 * SGW.exe - GASGlobalContext (1 functions)
 */

GRefCountBaseImpl * __thiscall GASGlobalContext__vfunc_0(void *this,uint param_1)

{
  FUN_0141c520(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASKeyAsObject.c ========== */

/*
 * SGW.exe - GASKeyAsObject (1 functions)
 */

CBitmapButton * __thiscall GASKeyAsObject__vfunc_0(void *this,uint param_1)

{
  CBitmapButton::~CBitmapButton(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASKeyAsObject_KeyListener.c ========== */

/*
 * SGW.exe - GASKeyAsObject_KeyListener (1 functions)
 */

GRefCountBaseImpl * __thiscall GASKeyAsObject_KeyListener__vfunc_0(void *this,uint param_1)

{
  FUN_013f18a0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASLoadVarsProto.c ========== */

/*
 * SGW.exe - GASLoadVarsProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASLoadVarsProto__vfunc_0(void *this,uint param_1)

{
  FUN_014c7020(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASLocalFrame.c ========== */

/*
 * SGW.exe - GASLocalFrame (1 functions)
 */

GRefCountBaseImpl * __thiscall GASLocalFrame__vfunc_0(void *this,uint param_1)

{
  FUN_01426620(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASMatrixProto.c ========== */

/*
 * SGW.exe - GASMatrixProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASMatrixProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d5210(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASMouseCtorFunction.c ========== */

/*
 * SGW.exe - GASMouseCtorFunction (1 functions)
 */

GRefCountBaseImpl * __thiscall GASMouseCtorFunction__vfunc_0(void *this,uint param_1)

{
  FUN_0149e470(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASMouseProto.c ========== */

/*
 * SGW.exe - GASMouseProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASMouseProto__vfunc_0(void *this,uint param_1)

{
  FUN_0149e130(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASMovieClipLoader.c ========== */

/*
 * SGW.exe - GASMovieClipLoader (1 functions)
 */

GRefCountBaseImpl * __thiscall GASMovieClipLoader__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASMovieClipLoaderProto.c ========== */

/*
 * SGW.exe - GASMovieClipLoaderProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASMovieClipLoaderProto__vfunc_0(void *this,uint param_1)

{
  FUN_014c9010(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASMovieClipProto.c ========== */

/*
 * SGW.exe - GASMovieClipProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASMovieClipProto__vfunc_0(void *this,uint param_1)

{
  FUN_01461dd0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASNumberObject.c ========== */

/*
 * SGW.exe - GASNumberObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASNumberObject__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASNumberProto.c ========== */

/*
 * SGW.exe - GASNumberProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASNumberProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d9c40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASObject.c ========== */

/*
 * SGW.exe - GASObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASObject__vfunc_0(void *this,uint param_1)

{
  FID__unknown_013df890(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASObjectInterface.c ========== */

/*
 * SGW.exe - GASObjectInterface (1 functions)
 */

undefined4 * __thiscall GASObjectInterface__vfunc_0(void *this,uint param_1)

{
  FUN_01467600(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASObjectInterface_MemberVisitor.c ========== */

/*
 * SGW.exe - GASObjectInterface_MemberVisitor (1 functions)
 */

undefined4 * __thiscall GASObjectInterface_MemberVisitor__vfunc_0(void *this,uint param_1)

{
  FUN_01415100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GASPointProto.c ========== */

/*
 * SGW.exe - GASPointProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASPointProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d1c90(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASRectangleProto.c ========== */

/*
 * SGW.exe - GASRectangleProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASRectangleProto__vfunc_0(void *this,uint param_1)

{
  FUN_014a9910(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASSoundObject.c ========== */

/*
 * SGW.exe - GASSoundObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASSoundObject__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASSoundProto.c ========== */

/*
 * SGW.exe - GASSoundProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASSoundProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d8630(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASStageCtorFunction.c ========== */

/*
 * SGW.exe - GASStageCtorFunction (1 functions)
 */

GRefCountBaseImpl * __thiscall GASStageCtorFunction__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASStageProto.c ========== */

/*
 * SGW.exe - GASStageProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASStageProto__vfunc_0(void *this,uint param_1)

{
  FUN_014ce450(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASStringObject.c ========== */

/*
 * SGW.exe - GASStringObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASStringObject__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASStringProto.c ========== */

/*
 * SGW.exe - GASStringProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASStringProto__vfunc_0(void *this,uint param_1)

{
  FUN_014dbbb0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASSuperObject.c ========== */

/*
 * SGW.exe - GASSuperObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASSuperObject__vfunc_0(void *this,uint param_1)

{
  FUN_014258e0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTextField.c ========== */

/*
 * SGW.exe - GASTextField (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTextField__vfunc_0(void *this,uint param_1)

{
  FUN_013e7000(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTextFieldObject.c ========== */

/*
 * SGW.exe - GASTextFieldObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTextFieldObject__vfunc_0(void *this,uint param_1)

{
  FUN_013f6b00(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTextFieldProto.c ========== */

/*
 * SGW.exe - GASTextFieldProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTextFieldProto__vfunc_0(void *this,uint param_1)

{
  FUN_013f71c0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTextFormatObject.c ========== */

/*
 * SGW.exe - GASTextFormatObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTextFormatObject__vfunc_0(void *this,uint param_1)

{
  FUN_014a4950(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTextFormatProto.c ========== */

/*
 * SGW.exe - GASTextFormatProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTextFormatProto__vfunc_0(void *this,uint param_1)

{
  FUN_014a6570(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTransformObject.c ========== */

/*
 * SGW.exe - GASTransformObject (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTransformObject__vfunc_0(void *this,uint param_1)

{
  FUN_014d5a40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASTransformProto.c ========== */

/*
 * SGW.exe - GASTransformProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASTransformProto__vfunc_0(void *this,uint param_1)

{
  FUN_014d6ad0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASValueProperty.c ========== */

/*
 * SGW.exe - GASValueProperty (1 functions)
 */

XImage * __thiscall GASValueProperty__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XImage::~XImage(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASXml.c ========== */

/*
 * SGW.exe - GASXml (1 functions)
 */

GRefCountBaseImpl * __thiscall GASXml__vfunc_0(void *this,uint param_1)

{
  FUN_014c5b20(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASXmlNodeProto.c ========== */

/*
 * SGW.exe - GASXmlNodeProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASXmlNodeProto__vfunc_0(void *this,uint param_1)

{
  FUN_014c4fc0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASXmlProto.c ========== */

/*
 * SGW.exe - GASXmlProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASXmlProto__vfunc_0(void *this,uint param_1)

{
  FUN_014c6090(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASXmlSocket.c ========== */

/*
 * SGW.exe - GASXmlSocket (1 functions)
 */

GRefCountBaseImpl * __thiscall GASXmlSocket__vfunc_0(void *this,uint param_1)

{
  FUN_013dfc60(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GASXmlSocketProto.c ========== */

/*
 * SGW.exe - GASXmlSocketProto (1 functions)
 */

GRefCountBaseImpl * __thiscall GASXmlSocketProto__vfunc_0(void *this,uint param_1)

{
  FUN_013e0bb0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GAcquireInterface.c ========== */

/*
 * SGW.exe - GAcquireInterface (4 functions)
 */


/* public: virtual __thiscall GAcquireInterface::~GAcquireInterface(void) */

void __thiscall GAcquireInterface::~GAcquireInterface(GAcquireInterface *this)

{
                    /* 0x1cd70  18  ??1GAcquireInterface@@UAE@XZ */
  *(undefined ***)this = vftable;
  return;
}




/* public: virtual bool __thiscall GAcquireInterface::CanAcquire(void)
   Also in vtable: GDefaultAcquireInterface (slot 1)
   Also in vtable: GWaitable (slot 1)
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 1) */

bool __thiscall GAcquireInterface::CanAcquire(GAcquireInterface *this)

{
                    /* 0x51d30  49  ?CanAcquire@GAcquireInterface@@UAE_NXZ
                       0x51d30  68  ?Flush@GZLibFile@@UAE_NXZ
                       0x51d30  102  ?IsSignaled@GWaitable@@UBE_NXZ
                       0x51d30  155  ?TryAcquire@GAcquireInterface@@UAE_NXZ
                       0x51d30  159  ?TryAcquireCancel@GAcquireInterface@@UAE_NXZ
                       0x51d30  160  ?TryAcquireCancel@GEvent@@UAE_NXZ
                       0x51d30  163  ?TryAcquireCommit@GAcquireInterface@@UAE_NXZ
                       0x51d30  165  ?TryAcquireCommit@GMutex@@UAE_NXZ
                       0x51d30  166  ?TryAcquireCommit@GSemaphore@@UAE_NXZ */
  return true;
}




/* public: static bool __cdecl GAcquireInterface::AcquireMultipleObjects(class GWaitable *
   *,unsigned int,unsigned int) */

bool __cdecl
GAcquireInterface::AcquireMultipleObjects(GWaitable **param_1,uint param_2,uint param_3)

{
  bool bVar1;
  uint uVar2;
  int iVar3;
  int local_f8;
  bool local_ed;
  GEvent local_ec [60];
  undefined1 local_b0 [16];
  int local_a0 [35];
  uint local_14;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
                    /* 0x51fd0  40
                       ?AcquireMultipleObjects@GAcquireInterface@@SA_NPAPAVGWaitable@@II@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aac6;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_00452290(local_a0,(int)param_1,param_2);
  local_8 = 0;
  uVar2 = FUN_00451f10(local_a0[0],param_2);
  if ((uVar2 & 0xff) == 0) {
    if (param_3 == 0) {
      local_8 = 0xffffffff;
      FUN_00452310(local_a0);
      local_ed = false;
    }
    else {
      GEvent::GEvent(local_ec,false,false);
      local_8._0_1_ = 1;
      FUN_00452250(local_b0,param_1,param_2,local_ec,local_a0[0]);
      uVar2 = FUN_00451dd0(local_b0,FUN_00451ea0);
      if ((uVar2 & 0xff) == 0) {
        local_8 = (uint)local_8._1_3_ << 8;
        GEvent::~GEvent(local_ec);
        local_8 = 0xffffffff;
        FUN_00452310(local_a0);
        local_ed = false;
      }
      else {
        uVar2 = FUN_00451f10(local_a0[0],param_2);
        if ((uVar2 & 0xff) == 0) {
          local_ed = false;
          local_f8 = 0;
          local_14 = param_3;
          if (param_3 != 0xffffffff) {
            local_f8 = FUN_004560c0();
          }
          while (bVar1 = GEvent::Wait(local_ec,local_14), bVar1) {
            uVar2 = FUN_00451f10(local_a0[0],param_2);
            if ((uVar2 & 0xff) != 0) {
              local_ed = true;
              break;
            }
            if (param_3 != 0xffffffff) {
              iVar3 = FUN_004560c0();
              if (param_3 <= (uint)(iVar3 - local_f8)) break;
              local_14 = param_3 - (iVar3 - local_f8);
            }
          }
          FUN_00451e50(local_b0,FUN_00451ea0);
          local_8 = (uint)local_8._1_3_ << 8;
          GEvent::~GEvent(local_ec);
          local_8 = 0xffffffff;
          FUN_00452310(local_a0);
        }
        else {
          FUN_00451e50(local_b0,FUN_00451ea0);
          local_8 = (uint)local_8._1_3_ << 8;
          GEvent::~GEvent(local_ec);
          local_8 = 0xffffffff;
          FUN_00452310(local_a0);
          local_ed = true;
        }
      }
    }
  }
  else {
    local_8 = 0xffffffff;
    FUN_00452310(local_a0);
    local_ed = true;
  }
  ExceptionList = local_10;
  return local_ed;
}




/* public: static int __cdecl GAcquireInterface::AcquireOneOfMultipleObjects(class GWaitable *
   *,unsigned int,unsigned int) */

int __cdecl
GAcquireInterface::AcquireOneOfMultipleObjects(GWaitable **param_1,uint param_2,uint param_3)

{
  bool bVar1;
  uint uVar2;
  int iVar3;
  int local_70;
  uint local_64;
  GEvent local_60 [60];
  undefined1 local_24 [16];
  uint local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x52430  41
                       ?AcquireOneOfMultipleObjects@GAcquireInterface@@SAHPAPAVGWaitable@@II@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aaf8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  local_64 = FUN_004523c0((int)param_1,param_2);
  if (local_64 == 0xffffffff) {
    if (param_3 == 0) {
      local_64 = 0xffffffff;
    }
    else {
      GEvent::GEvent(local_60,false,false);
      local_8 = 0;
      FUN_00452250(local_24,param_1,param_2,local_60,0);
      uVar2 = FUN_00451dd0(local_24,FUN_00452340);
      if ((uVar2 & 0xff) == 0) {
        local_8 = 0xffffffff;
        GEvent::~GEvent(local_60);
        local_64 = 0;
      }
      else {
        local_64 = FUN_004523c0((int)param_1,param_2);
        if (local_64 == 0xffffffff) {
          local_70 = 0;
          local_14 = param_3;
          if (param_3 != 0xffffffff) {
            local_70 = FUN_004560c0();
          }
          local_64 = FUN_004523c0((int)param_1,param_2);
          if (local_64 == 0xffffffff) {
            while( true ) {
              do {
                bVar1 = GEvent::Wait(local_60,local_14);
                if ((!bVar1) ||
                   (local_64 = FUN_004523c0((int)param_1,param_2), local_64 != 0xffffffff))
                goto LAB_004525b7;
              } while (param_3 == 0xffffffff);
              iVar3 = FUN_004560c0();
              if (param_3 <= (uint)(iVar3 - local_70)) break;
              local_14 = param_3 - (iVar3 - local_70);
            }
          }
LAB_004525b7:
          FUN_00451e50(local_24,FUN_00452340);
          local_8 = 0xffffffff;
          GEvent::~GEvent(local_60);
        }
        else {
          FUN_00451e50(local_24,FUN_00452340);
          local_8 = 0xffffffff;
          GEvent::~GEvent(local_60);
        }
      }
    }
  }
  ExceptionList = local_10;
  return local_64;
}




/* === Additional global functions for GAcquireInterface (1 functions) === */

/* Also in vtable: GAcquireInterface (slot 0) */

undefined4 * __thiscall GAcquireInterface__vfunc_0(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = GAcquireInterface::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,4,*(int *)((int)this + -4),GAcquireInterface::~GAcquireInterface);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}




/* ========== GAllocator.c ========== */

/*
 * SGW.exe - GAllocator (1 functions)
 */

/* Also in vtable: GAllocator (slot 0) */

undefined4 * __thiscall GAllocator__vfunc_0(void *this,uint param_1)

{
  FUN_00455c90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GBufferedFile.c ========== */

/*
 * SGW.exe - GBufferedFile (20 functions)
 */


/* protected: __thiscall GBufferedFile::GBufferedFile(void) */

GBufferedFile * __thiscall GBufferedFile::GBufferedFile(GBufferedFile *this)

{
  undefined4 uVar1;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfcd9f0  1  ??0GBufferedFile@@IAE@XZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e258;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_013cda80(this,0);
  local_8 = 0;
  *(undefined ***)this = vftable;
  uVar1 = FUN_00455b70(0x1ff8,0x20,0);
  *(undefined4 *)(this + 0x14) = uVar1;
  *(undefined4 *)(this + 0x18) = 0;
  *(undefined4 *)(this + 0x24) = 0;
  *(undefined4 *)(this + 0x28) = 0;
  ExceptionList = local_10;
  return this;
}




/* public: __thiscall GBufferedFile::GBufferedFile(class GFile *) */

GBufferedFile * __thiscall GBufferedFile::GBufferedFile(GBufferedFile *this,GFile *param_1)

{
  uint uVar1;
  undefined4 uVar2;
  undefined8 uVar3;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfcdf70  2  ??0GBufferedFile@@QAE@PAVGFile@@@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e258;
  local_10 = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xfffffffc;
  ExceptionList = &local_10;
  FUN_013cda80(this,(int)param_1);
  local_8 = 0;
  *(undefined ***)this = vftable;
  uVar2 = FUN_00455b70(0x1ff8,0x20,0);
  *(undefined4 *)(this + 0x14) = uVar2;
  *(undefined4 *)(this + 0x18) = 0;
  uVar3 = (**(code **)(*(int *)param_1 + 0x14))(uVar1);
  *(undefined8 *)(this + 0x24) = uVar3;
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GBufferedFile::~GBufferedFile(void) */

void __thiscall GBufferedFile::~GBufferedFile(GBufferedFile *this)

{
  int iVar1;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfce010  19  ??1GBufferedFile@@UAE@XZ */
  puStack_c = &LAB_0177e258;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  local_8 = 0;
  iVar1 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  if (iVar1 != 0) {
    FlushBuffer(this);
  }
  if (*(int *)(this + 0x14) != 0) {
    FUN_00455ba0(*(undefined4 *)(this + 0x14));
  }
  local_8 = 0xffffffff;
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>
            ((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* protected: bool __thiscall GBufferedFile::SetBufferMode(enum GBufferedFile::BufferModeType) */

bool __thiscall GBufferedFile::SetBufferMode(GBufferedFile *this,BufferModeType param_1)

{
  char cVar1;
  bool bVar2;
  int iVar3;
  int *piVar4;
  
                    /* 0xfce090  133  ?SetBufferMode@GBufferedFile@@IAE_NW4BufferModeType@1@@Z */
  if (*(int *)(this + 0x14) == 0) {
    return false;
  }
  if (param_1 != *(BufferModeType *)(this + 0x18)) {
    FlushBuffer(this);
  }
  if (param_1 == 2) {
    iVar3 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    if (iVar3 != 0) {
      piVar4 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
      cVar1 = (**(code **)(*piVar4 + 0xc))();
      if (cVar1 != '\0') goto LAB_013ce0f4;
    }
    bVar2 = false;
  }
  else {
LAB_013ce0f4:
    *(BufferModeType *)(this + 0x18) = param_1;
    *(undefined4 *)(this + 0x1c) = 0;
    *(undefined4 *)(this + 0x20) = 0;
    bVar2 = true;
  }
  return bVar2;
}




/* protected: void __thiscall GBufferedFile::FlushBuffer(void) */

void __thiscall GBufferedFile::FlushBuffer(GBufferedFile *this)

{
  uint uVar1;
  int *piVar2;
  uint uVar3;
  undefined8 uVar4;
  
                    /* 0xfce120  69  ?FlushBuffer@GBufferedFile@@IAEXXZ */
  if (*(int *)(this + 0x18) == 1) {
    if (*(int *)(this + 0x20) != *(int *)(this + 0x1c) &&
        -1 < *(int *)(this + 0x20) - *(int *)(this + 0x1c)) {
      piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
      uVar4 = (**(code **)(*piVar2 + 0x3c))
                        (-(*(int *)(this + 0x20) - *(int *)(this + 0x1c)),
                         -(*(int *)(this + 0x20) - *(int *)(this + 0x1c)) >> 0x1f,1);
      *(undefined8 *)(this + 0x24) = uVar4;
    }
    *(undefined4 *)(this + 0x20) = 0;
    *(undefined4 *)(this + 0x1c) = 0;
  }
  else if (*(int *)(this + 0x18) == 2) {
    piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    uVar3 = (**(code **)(*piVar2 + 0x24))(*(undefined4 *)(this + 0x14),*(undefined4 *)(this + 0x1c))
    ;
    uVar1 = *(uint *)(this + 0x24);
    *(uint *)(this + 0x24) = uVar3 + *(uint *)(this + 0x24);
    *(uint *)(this + 0x28) =
         *(int *)(this + 0x28) + ((int)uVar3 >> 0x1f) + (uint)CARRY4(uVar3,uVar1);
    *(undefined4 *)(this + 0x1c) = 0;
  }
  return;
}




/* protected: void __thiscall GBufferedFile::LoadBuffer(void) */

void __thiscall GBufferedFile::LoadBuffer(GBufferedFile *this)

{
  int *piVar1;
  uint uVar2;
  
                    /* 0xfce1f0  113  ?LoadBuffer@GBufferedFile@@IAEXXZ */
  if (*(int *)(this + 0x18) == 1) {
    piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    uVar2 = (**(code **)(*piVar1 + 0x28))(*(undefined4 *)(this + 0x14),0x1ff8);
    *(uint *)(this + 0x20) = ((int)uVar2 < 0) - 1 & uVar2;
    *(undefined4 *)(this + 0x1c) = 0;
    uVar2 = *(uint *)(this + 0x24);
    *(uint *)(this + 0x24) = *(uint *)(this + 0x20) + *(uint *)(this + 0x24);
    *(uint *)(this + 0x28) = *(int *)(this + 0x28) + (uint)CARRY4(*(uint *)(this + 0x20),uVar2);
  }
  return;
}




/* public: virtual int __thiscall GBufferedFile::Tell(void) */

int __thiscall GBufferedFile::Tell(GBufferedFile *this)

{
  int *piVar1;
  int iVar2;
  
                    /* 0xfce270  150  ?Tell@GBufferedFile@@UAEHXZ */
  if (*(int *)(this + 0x18) == 1) {
    iVar2 = (*(int *)(this + 0x24) - *(int *)(this + 0x20)) + *(int *)(this + 0x1c);
  }
  else {
    piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    iVar2 = (**(code **)(*piVar1 + 0x10))();
    if (iVar2 != -1) {
      if (*(int *)(this + 0x18) == 1) {
        iVar2 = iVar2 - (*(int *)(this + 0x20) - *(int *)(this + 0x1c));
      }
      else if (*(int *)(this + 0x18) == 2) {
        iVar2 = iVar2 + *(int *)(this + 0x1c);
      }
    }
  }
  return iVar2;
}




/* public: virtual __int64 __thiscall GBufferedFile::LTell(void) */

__int64 __thiscall GBufferedFile::LTell(GBufferedFile *this)

{
  int *piVar1;
  uint uVar2;
  longlong lVar3;
  
                    /* 0xfce310  111  ?LTell@GBufferedFile@@UAE_JXZ */
  if (*(int *)(this + 0x18) == 1) {
    uVar2 = *(uint *)(this + 0x24) - *(uint *)(this + 0x20);
    lVar3 = CONCAT44((*(int *)(this + 0x28) -
                     (uint)(*(uint *)(this + 0x24) < *(uint *)(this + 0x20))) +
                     (uint)CARRY4(uVar2,*(uint *)(this + 0x1c)),uVar2 + *(uint *)(this + 0x1c));
  }
  else {
    piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    lVar3 = (**(code **)(*piVar1 + 0x14))();
    if (lVar3 != -1) {
      if (*(int *)(this + 0x18) == 1) {
        lVar3 = CONCAT44((int)((ulonglong)lVar3 >> 0x20) -
                         (uint)((uint)lVar3 < (uint)(*(int *)(this + 0x20) - *(int *)(this + 0x1c)))
                         ,(uint)lVar3 - (*(int *)(this + 0x20) - *(int *)(this + 0x1c)));
      }
      else if (*(int *)(this + 0x18) == 2) {
        lVar3 = lVar3 + (ulonglong)*(uint *)(this + 0x1c);
      }
    }
  }
  return lVar3;
}




/* public: virtual int __thiscall GBufferedFile::GetLength(void) */

int __thiscall GBufferedFile::GetLength(GBufferedFile *this)

{
  int *piVar1;
  int iVar2;
  int local_8;
  
                    /* 0xfce3d0  88  ?GetLength@GBufferedFile@@UAEHXZ */
  piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  local_8 = (**(code **)(*piVar1 + 0x18))();
  if ((local_8 != -1) && (*(int *)(this + 0x18) == 2)) {
    piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    iVar2 = (**(code **)(*piVar1 + 0x10))();
    if (local_8 < iVar2 + *(int *)(this + 0x1c)) {
      local_8 = iVar2 + *(int *)(this + 0x1c);
    }
  }
  return local_8;
}




/* WARNING: Removing unreachable block (ram,0x013ce4b6) */
/* public: virtual __int64 __thiscall GBufferedFile::LGetLength(void) */

__int64 __thiscall GBufferedFile::LGetLength(GBufferedFile *this)

{
  int *piVar1;
  longlong lVar2;
  longlong lVar3;
  
                    /* 0xfce440  107  ?LGetLength@GBufferedFile@@UAE_JXZ */
  piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  lVar2 = (**(code **)(*piVar1 + 0x1c))();
  if ((lVar2 != -1) && (*(int *)(this + 0x18) == 2)) {
    piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    lVar3 = (**(code **)(*piVar1 + 0x14))();
    if (lVar2 < (longlong)(lVar3 + (ulonglong)*(uint *)(this + 0x1c))) {
      lVar2 = lVar3 + (ulonglong)*(uint *)(this + 0x1c);
    }
  }
  return lVar2;
}




/* public: virtual int __thiscall GBufferedFile::Write(unsigned char const *,int) */

int __thiscall GBufferedFile::Write(GBufferedFile *this,uchar *param_1,int param_2)

{
  uint uVar1;
  bool bVar2;
  int *piVar3;
  
                    /* 0xfce4e0  172  ?Write@GBufferedFile@@UAEHPBEH@Z */
  if ((*(int *)(this + 0x18) == 2) || (bVar2 = SetBufferMode(this,2), bVar2)) {
    if ((0x1ff8 - *(int *)(this + 0x1c) < param_2) && (FlushBuffer(this), 0x1000 < param_2)) {
      piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
      param_2 = (**(code **)(*piVar3 + 0x24))(param_1,param_2);
      if (0 < param_2) {
        uVar1 = *(uint *)(this + 0x24);
        *(uint *)(this + 0x24) = param_2 + *(uint *)(this + 0x24);
        *(uint *)(this + 0x28) =
             *(int *)(this + 0x28) + (param_2 >> 0x1f) + (uint)CARRY4(param_2,uVar1);
      }
    }
    else {
      memcpy((void *)(*(int *)(this + 0x14) + *(int *)(this + 0x1c)),param_1,param_2);
      *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_2;
    }
  }
  else {
    piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    param_2 = (**(code **)(*piVar3 + 0x24))(param_1,param_2);
    if (0 < param_2) {
      uVar1 = *(uint *)(this + 0x24);
      *(uint *)(this + 0x24) = param_2 + *(uint *)(this + 0x24);
      *(uint *)(this + 0x28) =
           *(int *)(this + 0x28) + (param_2 >> 0x1f) + (uint)CARRY4(param_2,uVar1);
    }
  }
  return param_2;
}




/* public: virtual int __thiscall GBufferedFile::Read(unsigned char *,int) */

int __thiscall GBufferedFile::Read(GBufferedFile *this,uchar *param_1,int param_2)

{
  uint uVar1;
  bool bVar2;
  size_t _Size;
  int *piVar3;
  uint uVar4;
  
                    /* 0xfce5f0  123  ?Read@GBufferedFile@@UAEHPAEH@Z */
  if ((*(int *)(this + 0x18) == 1) || (bVar2 = SetBufferMode(this,1), bVar2)) {
    if (*(int *)(this + 0x20) - *(int *)(this + 0x1c) < param_2) {
      _Size = *(int *)(this + 0x20) - *(int *)(this + 0x1c);
      memcpy(param_1,(void *)(*(int *)(this + 0x14) + *(int *)(this + 0x1c)),_Size);
      param_2 = param_2 - _Size;
      *(undefined4 *)(this + 0x1c) = *(undefined4 *)(this + 0x20);
      if (param_2 < 0x1001) {
        LoadBuffer(this);
        if (*(int *)(this + 0x20) - *(int *)(this + 0x1c) < param_2) {
          param_2 = *(int *)(this + 0x20) - *(int *)(this + 0x1c);
        }
        memcpy(param_1 + _Size,(void *)(*(int *)(this + 0x14) + *(int *)(this + 0x1c)),param_2);
        *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_2;
        param_2 = param_2 + _Size;
      }
      else {
        piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
        uVar4 = (**(code **)(*piVar3 + 0x28))(param_1 + _Size,param_2);
        if (0 < (int)uVar4) {
          uVar1 = *(uint *)(this + 0x24);
          *(uint *)(this + 0x24) = uVar4 + *(uint *)(this + 0x24);
          *(uint *)(this + 0x28) =
               *(int *)(this + 0x28) + ((int)uVar4 >> 0x1f) + (uint)CARRY4(uVar4,uVar1);
        }
        param_2 = (-(uint)(uVar4 != 0xffffffff) & uVar4) + _Size;
      }
    }
    else {
      memcpy(param_1,(void *)(*(int *)(this + 0x14) + *(int *)(this + 0x1c)),param_2);
      *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_2;
    }
  }
  else {
    piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    param_2 = (**(code **)(*piVar3 + 0x28))(param_1,param_2);
    if (0 < param_2) {
      uVar4 = *(uint *)(this + 0x24);
      *(uint *)(this + 0x24) = param_2 + *(uint *)(this + 0x24);
      *(uint *)(this + 0x28) =
           *(int *)(this + 0x28) + (param_2 >> 0x1f) + (uint)CARRY4(param_2,uVar4);
    }
  }
  return param_2;
}




/* public: virtual int __thiscall GBufferedFile::SkipBytes(int) */

int __thiscall GBufferedFile::SkipBytes(GBufferedFile *this,int param_1)

{
  uint uVar1;
  int *piVar2;
  uint uVar3;
  int local_10;
  int local_8;
  
                    /* 0xfce7c0  144  ?SkipBytes@GBufferedFile@@UAEHH@Z */
  local_8 = 0;
  if (*(int *)(this + 0x18) == 1) {
    if (*(int *)(this + 0x20) - *(int *)(this + 0x1c) < param_1) {
      local_10 = *(int *)(this + 0x20) - *(int *)(this + 0x1c);
    }
    else {
      local_10 = param_1;
    }
    local_8 = local_10;
    *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + local_10;
    param_1 = param_1 - local_10;
  }
  if (param_1 != 0) {
    piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    uVar3 = (**(code **)(*piVar2 + 0x2c))(param_1);
    if (uVar3 == 0xffffffff) {
      if (local_8 < 1) {
        local_8 = -1;
      }
    }
    else {
      local_8 = local_8 + uVar3;
      uVar1 = *(uint *)(this + 0x24);
      *(uint *)(this + 0x24) = uVar3 + *(uint *)(this + 0x24);
      *(uint *)(this + 0x28) =
           *(int *)(this + 0x28) + ((int)uVar3 >> 0x1f) + (uint)CARRY4(uVar3,uVar1);
    }
  }
  return local_8;
}




/* public: virtual int __thiscall GBufferedFile::BytesAvailable(void) */

int __thiscall GBufferedFile::BytesAvailable(GBufferedFile *this)

{
  int *piVar1;
  int local_8;
  
                    /* 0xfce890  45  ?BytesAvailable@GBufferedFile@@UAEHXZ */
  piVar1 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  local_8 = (**(code **)(*piVar1 + 0x30))();
  if (*(int *)(this + 0x18) == 1) {
    local_8 = (*(int *)(this + 0x20) - *(int *)(this + 0x1c)) + local_8;
  }
  else if ((*(int *)(this + 0x18) == 2) && (local_8 = local_8 - *(int *)(this + 0x1c), local_8 < 0))
  {
    local_8 = 0;
  }
  return local_8;
}




/* public: virtual bool __thiscall GBufferedFile::Flush(void) */

bool __thiscall GBufferedFile::Flush(GBufferedFile *this)

{
  undefined1 uVar1;
  int *piVar2;
  
                    /* 0xfce900  67  ?Flush@GBufferedFile@@UAE_NXZ */
  FlushBuffer(this);
  piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  uVar1 = (**(code **)(*piVar2 + 0x34))();
  return (bool)uVar1;
}




/* public: virtual int __thiscall GBufferedFile::Seek(int,int) */

int __thiscall GBufferedFile::Seek(GBufferedFile *this,int param_1,int param_2)

{
  uint uVar1;
  int *piVar2;
  int iVar3;
  
                    /* 0xfce930  131  ?Seek@GBufferedFile@@UAEHHH@Z */
  if ((param_2 == 1) && ((uint)(param_1 + *(int *)(this + 0x1c)) < *(uint *)(this + 0x20))) {
    *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_1;
    iVar3 = (*(int *)(this + 0x24) - *(int *)(this + 0x20)) + *(int *)(this + 0x1c);
  }
  else {
    if (param_2 == 0) {
      uVar1 = (uint)(*(uint *)(this + 0x24) < *(uint *)(this + 0x20));
      if (((*(uint *)(this + 0x28) == uVar1) &&
          ((*(uint *)(this + 0x28) != uVar1 ||
           (*(uint *)(this + 0x24) - *(uint *)(this + 0x20) <= (uint)param_1)))) &&
         ((*(int *)(this + 0x28) != 0 || ((uint)param_1 < *(uint *)(this + 0x24))))) {
        *(int *)(this + 0x1c) = (param_1 - *(int *)(this + 0x24)) + *(int *)(this + 0x20);
        return (*(int *)(this + 0x24) - *(int *)(this + 0x20)) + *(int *)(this + 0x1c);
      }
    }
    FlushBuffer(this);
    piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    iVar3 = (**(code **)(*piVar2 + 0x38))(param_1,param_2);
    *(int *)(this + 0x24) = iVar3;
    *(int *)(this + 0x28) = iVar3 >> 0x1f;
    iVar3 = *(int *)(this + 0x24);
  }
  return iVar3;
}




/* WARNING: Removing unreachable block (ram,0x013ceb41) */
/* public: virtual __int64 __thiscall GBufferedFile::LSeek(__int64,int) */

__int64 __thiscall GBufferedFile::LSeek(GBufferedFile *this,__int64 param_1,int param_2)

{
  longlong lVar1;
  int *piVar2;
  int iVar3;
  uint uVar4;
  int iVar5;
  undefined8 uVar6;
  
                    /* 0xfcea90  109  ?LSeek@GBufferedFile@@UAE_J_JH@Z */
  if (param_2 == 1) {
    lVar1 = param_1 + (ulonglong)*(uint *)(this + 0x1c);
    if ((lVar1 < 0x100000000) && ((lVar1 < 0 || ((uint)lVar1 < *(uint *)(this + 0x20))))) {
      *(uint *)(this + 0x1c) = (uint)param_1 + *(int *)(this + 0x1c);
      uVar4 = *(uint *)(this + 0x24) - *(uint *)(this + 0x20);
      iVar5 = uVar4 + *(uint *)(this + 0x1c);
      iVar3 = (*(int *)(this + 0x28) - (uint)(*(uint *)(this + 0x24) < *(uint *)(this + 0x20))) +
              (uint)CARRY4(uVar4,*(uint *)(this + 0x1c));
      goto LAB_013cebee;
    }
  }
  if ((((param_2 == 0) &&
       (CONCAT44(*(int *)(this + 0x28) - (uint)(*(uint *)(this + 0x24) < *(uint *)(this + 0x20)),
                 *(uint *)(this + 0x24) - *(uint *)(this + 0x20)) <= param_1)) &&
      (param_1._4_4_ <= *(int *)(this + 0x28))) &&
     ((param_1._4_4_ < *(int *)(this + 0x28) || ((uint)param_1 < *(uint *)(this + 0x24))))) {
    *(uint *)(this + 0x1c) = ((uint)param_1 - *(int *)(this + 0x24)) + *(int *)(this + 0x20);
    uVar4 = *(uint *)(this + 0x24) - *(uint *)(this + 0x20);
    iVar5 = uVar4 + *(uint *)(this + 0x1c);
    iVar3 = (*(int *)(this + 0x28) - (uint)(*(uint *)(this + 0x24) < *(uint *)(this + 0x20))) +
            (uint)CARRY4(uVar4,*(uint *)(this + 0x1c));
  }
  else {
    FlushBuffer(this);
    piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    uVar6 = (**(code **)(*piVar2 + 0x3c))(param_1,param_2);
    *(undefined8 *)(this + 0x24) = uVar6;
    iVar5 = *(int *)(this + 0x24);
    iVar3 = *(int *)(this + 0x28);
  }
LAB_013cebee:
  return CONCAT44(iVar3,iVar5);
}




/* public: virtual bool __thiscall GBufferedFile::ChangeSize(int) */

bool __thiscall GBufferedFile::ChangeSize(GBufferedFile *this,int param_1)

{
  undefined1 uVar1;
  int *piVar2;
  
                    /* 0xfcec00  53  ?ChangeSize@GBufferedFile@@UAE_NH@Z */
  FlushBuffer(this);
  piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  uVar1 = (**(code **)(*piVar2 + 0x40))(param_1);
  return (bool)uVar1;
}




/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */
/* public: virtual int __thiscall GBufferedFile::CopyFromStream(class GFile *,int) */

int __thiscall GBufferedFile::CopyFromStream(GBufferedFile *this,GFile *param_1,int param_2)

{
  int iVar1;
  uint local_4020;
  int local_4014;
  undefined1 local_400c [16384];
  uint local_c;
  int local_8;
  
                    /* 0xfcec40  60  ?CopyFromStream@GBufferedFile@@UAEHPAVGFile@@H@Z */
  local_c = DAT_01e7f9a0 ^ (uint)&stack0xfffffffc;
  local_8 = 0;
  do {
    if (param_2 == 0) break;
    if (param_2 < 0x4001) {
      local_4020 = param_2;
    }
    else {
      local_4020 = (uint)(0x4000 < param_2);
    }
    iVar1 = (**(code **)(*(int *)param_1 + 0x28))(local_400c,local_4020);
    local_4014 = 0;
    if (0 < iVar1) {
      local_4014 = (**(code **)(*(int *)this + 0x24))(local_400c,iVar1);
    }
    local_8 = local_8 + local_4014;
    param_2 = param_2 - local_4014;
  } while ((int)local_4020 <= local_4014);
  iVar1 = __security_check_cookie(local_c ^ (uint)&stack0xfffffffc);
  return iVar1;
}




/* public: virtual bool __thiscall GBufferedFile::Close(void) */

bool __thiscall GBufferedFile::Close(GBufferedFile *this)

{
  undefined1 uVar1;
  int *piVar2;
  
                    /* 0xfced40  55  ?Close@GBufferedFile@@UAE_NXZ */
  if (*(int *)(this + 0x18) == 1) {
    *(undefined4 *)(this + 0x18) = 0;
  }
  else if (*(int *)(this + 0x18) == 2) {
    FlushBuffer(this);
  }
  piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  uVar1 = (**(code **)(*piVar2 + 0x48))();
  return (bool)uVar1;
}




/* === Additional global functions for GBufferedFile (1 functions) === */

GBufferedFile * __thiscall GBufferedFile__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GBufferedFile::~GBufferedFile(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_
              (this,0x2c,*(int *)((int)this + -4),GBufferedFile::~GBufferedFile);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== GColor.c ========== */

/*
 * SGW.exe - GColor (12 functions)
 */


/* public: void __thiscall GColor::Format(char *)const  */

void __thiscall GColor::Format(GColor *this,char *param_1)

{
                    /* 0x10740a0  70  ?Format@GColor@@QBEXPAD@Z */
  FUN_013cab40(param_1,0x200,"RGBA: %d %d %d %d\n");
  return;
}




/* public: static class GColor __stdcall GColor::Blend(class GColor,class GColor,float) */

void * GColor::Blend(void *param_1,uint param_2,uint param_3,float param_4)

{
  undefined4 extraout_ECX;
  undefined4 extraout_ECX_00;
  undefined4 extraout_ECX_01;
  undefined4 extraout_ECX_02;
  undefined4 extraout_EDX;
  undefined4 extraout_EDX_00;
  undefined4 extraout_EDX_01;
  undefined4 extraout_EDX_02;
  ulonglong uVar1;
  undefined4 local_8;
  
                    /* 0x10740f0  44  ?Blend@GColor@@SG?AV1@V1@0M@Z */
  FBSPSurfaceStaticLighting__vfunc_0(&local_8);
  GSemaphoreWaitableIncrement__unknown_00af2120
            ((float)(param_2 >> 0x10 & 0xff),(float)(param_3 >> 0x10 & 0xff),param_4);
  uVar1 = GFxSprite__unknown_013e2720(extraout_ECX,extraout_EDX);
  local_8._2_1_ = (undefined1)uVar1;
  GSemaphoreWaitableIncrement__unknown_00af2120
            ((float)(param_2 >> 8 & 0xff),(float)(param_3 >> 8 & 0xff),param_4);
  uVar1 = GFxSprite__unknown_013e2720(extraout_ECX_00,extraout_EDX_00);
  local_8._1_1_ = (undefined1)uVar1;
  GSemaphoreWaitableIncrement__unknown_00af2120
            ((float)(param_2 & 0xff),(float)(param_3 & 0xff),param_4);
  uVar1 = GFxSprite__unknown_013e2720(extraout_ECX_01,extraout_EDX_01);
  local_8._0_1_ = (undefined1)uVar1;
  GSemaphoreWaitableIncrement__unknown_00af2120
            ((float)(param_2 >> 0x18),(float)(param_3 >> 0x18),param_4);
  uVar1 = GFxSprite__unknown_013e2720(extraout_ECX_02,extraout_EDX_02);
  local_8._3_1_ = (undefined1)uVar1;
  GFxSprite__unknown_00af2090(param_1,&local_8);
  return param_1;
}




/* public: void __thiscall GColor::SetHSV(int,int,int) */

void __thiscall GColor::SetHSV(GColor *this,int param_1,int param_2,int param_3)

{
  undefined1 uVar1;
  undefined1 uVar2;
  undefined1 uVar3;
  uint uVar4;
  undefined1 local_10;
  undefined1 local_c;
  undefined1 local_8;
  
                    /* 0x1074200  138  ?SetHSV@GColor@@QAEXHHH@Z */
  local_10 = (undefined1)param_3;
  local_8 = local_10;
  local_c = local_10;
  uVar1 = local_10;
  if ((param_2 != 0) && (local_8 = local_10, local_c = local_10, -1 < param_1)) {
    if (0x167 < param_1) {
      param_1 = param_1 % 0x168;
    }
    uVar4 = param_1 / 0x3c;
    uVar2 = (undefined1)((param_3 * 2 * (0xff - param_2) + 0xffU) / 0x1fe);
    local_8 = uVar2;
    if ((uVar4 & 1) == 0) {
      uVar3 = (undefined1)
              ((param_3 * 2 * (0x3bc4 - (0x3c - param_1 % 0x3c) * param_2) + 0x3bc4U) / 0x7788);
      local_c = uVar3;
      if (((uVar4 != 0) && (local_8 = uVar3, local_c = local_10, uVar1 = uVar2, uVar4 != 2)) &&
         (local_8 = local_10, local_c = uVar2, uVar1 = uVar3, uVar4 != 4)) {
        local_8 = local_10;
        local_c = local_10;
        uVar1 = local_10;
      }
    }
    else {
      uVar3 = (undefined1)((param_3 * 2 * (0x3bc4 - param_2 * (param_1 % 0x3c)) + 0x3bc4U) / 0x7788)
      ;
      local_c = local_10;
      uVar1 = uVar3;
      if (((uVar4 != 1) && (local_8 = local_10, local_c = uVar3, uVar1 = uVar2, uVar4 != 3)) &&
         (local_8 = uVar3, local_c = uVar2, uVar1 = local_10, uVar4 != 5)) {
        local_8 = local_10;
        local_c = local_10;
      }
    }
  }
  local_10 = uVar1;
  FUN_013c8b80(this,local_10,local_c,local_8);
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: void __thiscall GColor::SetHSV(float,float,float) */

void __thiscall GColor::SetHSV(GColor *this,float param_1,float param_2,float param_3)

{
  float fVar1;
  float fVar2;
  undefined4 extraout_ECX;
  undefined4 in_EDX;
  ulonglong uVar3;
  float local_1c;
  float local_18;
  float local_14;
  
                    /* 0x10743b0  139  ?SetHSV@GColor@@QAEXMMM@Z */
  if (param_2 == (float)_DAT_017f9268) {
    local_14 = param_3;
    local_18 = param_3;
    local_1c = param_3;
  }
  else {
    if (param_1 == (float)DAT_017fbc18) {
      param_1 = 0.0;
    }
    else {
      param_1 = param_1 * (float)_PTR_018efab0;
    }
    uVar3 = CEGUI__unknown_012379c0(this,in_EDX);
    fVar1 = param_1 - (float)(int)uVar3;
    local_1c = (1.0 - param_2) * param_3;
    fVar2 = (1.0 - param_2 * fVar1) * param_3;
    fVar1 = (1.0 - (1.0 - fVar1) * param_2) * param_3;
    uVar3 = CEGUI__unknown_012379c0(extraout_ECX,(int)(uVar3 >> 0x20));
    local_18 = local_1c;
    local_14 = local_1c;
    switch((int)uVar3) {
    case 0:
      local_1c = param_3;
      local_18 = fVar1;
      break;
    case 1:
      local_18 = param_3;
      local_1c = fVar2;
      break;
    case 2:
      local_18 = param_3;
      local_14 = fVar1;
      break;
    case 3:
      local_14 = param_3;
      local_18 = fVar2;
      break;
    case 4:
      local_14 = param_3;
      local_1c = fVar1;
      break;
    default:
      local_1c = param_3;
      local_14 = fVar2;
    }
  }
  FUN_01474520(this,local_1c,local_18,local_14);
  return;
}




/* public: void __thiscall GColor::GetHSV(int *,int *,int *)const  */

void __thiscall GColor::GetHSV(GColor *this,int *param_1,int *param_2,int *param_3)

{
  char cVar1;
  uint uVar2;
  uint uVar3;
  int iVar4;
  uint uVar5;
  uint local_34;
  byte local_21 [12];
  byte local_15;
  uint local_14;
  uint local_10;
  byte local_9 [5];
  
                    /* 0x10745c0  86  ?GetHSV@GColor@@QBEXPAH00@Z */
  FUN_014747b0(this,local_9,local_21,&local_15);
  uVar2 = (uint)local_9[0];
  uVar3 = (uint)local_21[0];
  uVar5 = (uint)local_15;
  cVar1 = uVar2 < uVar3;
  local_10 = uVar2;
  if ((bool)cVar1) {
    local_10 = uVar3;
  }
  if (local_10 < uVar5) {
    cVar1 = '\x02';
    local_10 = uVar5;
  }
  local_14 = uVar2;
  if (uVar3 < uVar2) {
    local_14 = uVar3;
  }
  if (uVar5 < local_14) {
    local_14 = uVar5;
  }
  iVar4 = local_10 - local_14;
  *param_3 = local_10;
  if (local_10 == 0) {
    local_34 = 0;
  }
  else {
    local_34 = (iVar4 * 0x1fe + local_10) / (local_10 << 1);
  }
  *param_2 = local_34;
  if (*param_2 == 0) {
    *param_1 = 0;
  }
  else if (cVar1 == '\0') {
    if (uVar3 < uVar5) {
      *param_1 = (int)((uVar3 - uVar5) * 0x78 + iVar4 * 0x79) / (iVar4 * 2) + 300;
    }
    else {
      *param_1 = (int)((uVar3 - uVar5) * 0x78 + iVar4) / (iVar4 * 2);
    }
  }
  else if (cVar1 == '\x01') {
    if (uVar2 < uVar5) {
      *param_1 = (int)((uVar5 - uVar2) * 0x78 + iVar4) / (iVar4 * 2) + 0x78;
    }
    else {
      *param_1 = (int)((uVar5 - uVar2) * 0x78 + iVar4 * 0x79) / (iVar4 * 2) + 0x3c;
    }
  }
  else if (cVar1 == '\x02') {
    if (uVar3 < uVar2) {
      *param_1 = (int)((uVar2 - uVar3) * 0x78 + iVar4) / (iVar4 * 2) + 0xf0;
    }
    else {
      *param_1 = (int)((uVar2 - uVar3) * 0x78 + iVar4 * 0x79) / (iVar4 * 2) + 0xb4;
    }
  }
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: void __thiscall GColor::GetHSV(float *,float *,float *)const  */

void __thiscall GColor::GetHSV(GColor *this,float *param_1,float *param_2,float *param_3)

{
  float fVar1;
  float10 fVar2;
  float local_1c [2];
  float local_14;
  float local_10;
  float local_c;
  
                    /* 0x10747f0  87  ?GetHSV@GColor@@QBEXPAM00@Z */
  FUN_00af20a0(this,local_1c,&local_14,&local_10);
  fVar2 = FUN_00aef3b0(local_14,local_10);
  fVar2 = FUN_00aef3b0(local_1c[0],(float)fVar2);
  local_c = (float)fVar2;
  fVar2 = GFxTextDocView__unknown_00aef3e0(local_14,local_10);
  fVar2 = GFxTextDocView__unknown_00aef3e0(local_1c[0],(float)fVar2);
  fVar1 = (float)fVar2;
  local_c = fVar1 - local_c;
  *param_3 = fVar1;
  if (fVar1 == (float)_DAT_017f9268) {
    *param_2 = 0.0;
    *param_1 = 0.0;
  }
  else {
    *param_2 = local_c / fVar1;
    if (*param_2 == (float)_DAT_017f9268) {
      *param_1 = 0.0;
    }
    else {
      if (fVar1 == local_1c[0]) {
        *param_1 = (local_14 - local_10) / local_c;
      }
      else if (fVar1 == local_14) {
        *param_1 = (local_10 - local_1c[0]) / local_c + (float)DAT_01866fe0;
      }
      else {
        *param_1 = (local_1c[0] - local_14) / local_c + (float)_DAT_0193f230;
      }
      *param_1 = *param_1 / (float)_PTR_018efab0;
      if (*param_1 < (float)_DAT_017f9268) {
        *param_1 = *param_1 + (float)DAT_017fbc18;
      }
      if ((float)DAT_017fbc18 < *param_1) {
        *param_1 = *param_1 - (float)DAT_017fbc18;
      }
    }
  }
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: void __thiscall GColor::SetHSI(int,int,int) */

void __thiscall GColor::SetHSI(GColor *this,int param_1,int param_2,int param_3)

{
                    /* 0x1074980  136  ?SetHSI@GColor@@QAEXHHH@Z */
  SetHSI(this,(float)param_1 / (float)_PTR_018a20b8,(float)param_2 / (float)_DAT_01aa8d20,
         (float)param_3 / (float)_DAT_01aa8d20);
  return;
}




/* public: void __thiscall GColor::GetHSI(int *,int *,int *)const  */

void __thiscall GColor::GetHSI(GColor *this,int *param_1,int *param_2,int *param_3)

{
  undefined4 extraout_ECX;
  undefined4 extraout_ECX_00;
  undefined4 extraout_EDX;
  ulonglong uVar1;
  float local_10;
  float local_c;
  float local_8;
  
                    /* 0x10749d0  84  ?GetHSI@GColor@@QBEXPAH00@Z */
  GetHSI(this,&local_8,&local_10,&local_c);
  uVar1 = CEGUI__unknown_012379c0(extraout_ECX,extraout_EDX);
  *param_1 = (int)uVar1;
  uVar1 = CEGUI__unknown_012379c0(param_1,(int)(uVar1 >> 0x20));
  *param_2 = (int)uVar1;
  uVar1 = CEGUI__unknown_012379c0(extraout_ECX_00,param_2);
  *param_3 = (int)uVar1;
  return;
}




/* public: void __thiscall GColor::SetHSI(float,float,float) */

void __thiscall GColor::SetHSI(GColor *this,float param_1,float param_2,float param_3)

{
  double local_1c;
  double local_14;
  double local_c;
  
                    /* 0x1074a30  137  ?SetHSI@GColor@@QAEXMMM@Z */
  ConvertHSIToRGB((double)param_1,(double)param_2,(double)param_3,&local_1c,&local_14,&local_c);
  FUN_01474520(this,(float)local_1c,(float)local_14,(float)local_c);
  return;
}




/* public: void __thiscall GColor::GetHSI(float *,float *,float *)const  */

void __thiscall GColor::GetHSI(GColor *this,float *param_1,float *param_2,float *param_3)

{
  double local_2c;
  float local_20;
  double local_1c;
  float local_14;
  float local_10;
  double local_c;
  
                    /* 0x1074aa0  85  ?GetHSI@GColor@@QBEXPAM00@Z */
  FUN_00af20a0(this,&local_20,&local_14,&local_10);
  ConvertRGBToHSI((double)local_20,(double)local_14,(double)local_10,&local_1c,&local_c,&local_2c);
  *param_1 = (float)local_1c;
  *param_2 = (float)local_c;
  *param_3 = (float)local_2c;
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: static void __stdcall GColor::ConvertRGBToHSI(double,double,double,double *,double
   *,double *) */

void GColor::ConvertRGBToHSI
               (double param_1,double param_2,double param_3,double *param_4,double *param_5,
               double *param_6)

{
  double dVar1;
  float10 fVar2;
  double dVar3;
  double dVar4;
  undefined8 local_3c;
  undefined8 local_2c;
  undefined8 local_c;
  
                    /* 0x1074b10  59  ?ConvertRGBToHSI@GColor@@SGXNNNPAN00@Z */
  dVar1 = (param_1 + param_2 + param_3) / _DAT_0183db60;
  if (dVar1 == 0.0) {
    local_2c = 1.0;
  }
  else {
    dVar4 = param_3;
    fVar2 = FUN_01474e60(param_1,param_2);
    fVar2 = FUN_01474e60((double)fVar2,dVar4);
    local_2c = (double)((float10)1 - fVar2 / (float10)dVar1);
  }
  if ((param_2 == param_1) && (param_3 == param_2)) {
    local_c = 0.0;
  }
  else {
    dVar4 = (((param_1 - param_2) + param_1) - param_3) * _PTR_0181cc88;
    dVar3 = sqrt((param_2 - param_3) * (param_1 - param_3) +
                 (param_1 - param_2) * (param_1 - param_2));
    local_3c = acos(dVar4 / dVar3);
    if (param_2 <= param_3) {
      local_3c = _DAT_01866fe8 - local_3c;
    }
    local_c = local_3c;
  }
  *param_4 = local_c;
  *param_5 = local_2c;
  *param_6 = dVar1;
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: static void __stdcall GColor::ConvertHSIToRGB(double,double,double,double *,double
   *,double *) */

void GColor::ConvertHSIToRGB
               (double param_1,double param_2,double param_3,double *param_4,double *param_5,
               double *param_6)

{
  double dVar1;
  double dVar2;
  undefined8 local_24;
  undefined8 local_14;
  undefined8 local_c;
  
                    /* 0x1074c40  58  ?ConvertHSIToRGB@GColor@@SGXNNNPAN00@Z */
  if (param_1 == 0.0) {
    local_c = param_3;
    local_14 = param_3;
    local_24 = param_3;
  }
  else if ((param_1 <= 0.0) || (_DAT_01af4118 <= param_1)) {
    if ((_DAT_01af4118 < param_1 == (_DAT_01af4118 == param_1)) || (_DAT_01af4108 <= param_1)) {
      dVar1 = sqrt(_DAT_0183db60);
      dVar2 = tan(param_1 - _DAT_01af4100);
      dVar2 = dVar2 * (1.0 / dVar1);
      local_14 = (1.0 - param_2) * param_3;
      local_24 = (_DAT_01849300 * dVar2 + _DAT_01849300) * param_3 -
                 (_DAT_01849300 * dVar2 + _PTR_0181cc88) * local_14;
      local_c = (_DAT_0183db60 * param_3 - local_24) - local_14;
    }
    else {
      dVar1 = sqrt(_DAT_0183db60);
      dVar2 = tan(param_1 - DAT_01816ac8);
      dVar2 = dVar2 * (1.0 / dVar1);
      local_24 = (1.0 - param_2) * param_3;
      local_c = (_DAT_01849300 * dVar2 + _DAT_01849300) * param_3 -
                (_DAT_01849300 * dVar2 + _PTR_0181cc88) * local_24;
      local_14 = (_DAT_0183db60 * param_3 - local_c) - local_24;
    }
  }
  else {
    dVar1 = sqrt(_DAT_0183db60);
    dVar2 = tan(param_1 - _DAT_01af4110);
    dVar2 = dVar2 * (1.0 / dVar1);
    local_c = (1.0 - param_2) * param_3;
    local_14 = (_DAT_01849300 * dVar2 + _DAT_01849300) * param_3 -
               (_DAT_01849300 * dVar2 + _PTR_0181cc88) * local_c;
    local_24 = (_DAT_0183db60 * param_3 - local_14) - local_c;
  }
  *param_4 = local_24;
  *param_5 = local_14;
  *param_6 = local_c;
  return;
}





/* ========== GDebugAllocator.c ========== */

/*
 * SGW.exe - GDebugAllocator (5 functions)
 */

/* Also in vtable: GDebugAllocator (slot 0) */

undefined4 * __thiscall GDebugAllocator__vfunc_0(void *this,uint param_1)

{
  FUN_00455c10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] GDebugAllocator virtual function #1
   VTable: vtable_GDebugAllocator at 017fbc34 */

void GDebugAllocator__vfunc_1(int param_1,int param_2)

{
  undefined4 *puVar1;
  
  if (param_1 != 0) {
    puVar1 = GDebugAllocator__unknown_00457b40(param_1,param_2);
    if (DAT_01ea5620 == (int *)0x0) {
      DAT_01ea5620 = (int *)GDebugAllocator__unknown_00455ae0();
    }
    (**(code **)(*DAT_01ea5620 + 8))(puVar1);
  }
  return;
}


/* [VTable] GDebugAllocator virtual function #4
   VTable: vtable_GDebugAllocator at 017fbc34 */

void GDebugAllocator__vfunc_4(int param_1,int param_2)

{
  undefined4 *puVar1;
  
  if (param_1 != 0) {
    puVar1 = GDebugAllocator__unknown_00457b40(param_1,param_2);
    if (DAT_01ea5620 == (int *)0x0) {
      DAT_01ea5620 = (int *)GDebugAllocator__unknown_00455ae0();
    }
    (**(code **)(*DAT_01ea5620 + 0x18))(puVar1);
  }
  return;
}


/* [VTable] GDebugAllocator virtual function #6
   VTable: vtable_GDebugAllocator at 017fbc34 */

void __fastcall GDebugAllocator__vfunc_6(undefined4 param_1)

{
  if (DAT_01ea5620 == (int *)0x0) {
    DAT_01ea5620 = (int *)GDebugAllocator__unknown_00455ae0();
  }
  (**(code **)(*DAT_01ea5620 + 0x10))(param_1);
  return;
}


/* [VTable] GDebugAllocator virtual function #7
   VTable: vtable_GDebugAllocator at 017fbc34 */

void GDebugAllocator__vfunc_7(void)

{
  FUN_00457d50();
  return;
}




/* ========== GDefaultAcquireInterface.c ========== */

/*
 * SGW.exe - GDefaultAcquireInterface (2 functions)
 */


/* public: virtual __thiscall GDefaultAcquireInterface::~GDefaultAcquireInterface(void) */

void __thiscall GDefaultAcquireInterface::~GDefaultAcquireInterface(GDefaultAcquireInterface *this)

{
                    /* 0x1cde0  20  ??1GDefaultAcquireInterface@@UAE@XZ */
  *(undefined ***)this = vftable;
  *(undefined ***)this = GAcquireInterface::vftable;
  return;
}




/* public: static class GDefaultAcquireInterface * __cdecl
   GDefaultAcquireInterface::GetDefaultAcquireInterface(void) */

GDefaultAcquireInterface * __cdecl GDefaultAcquireInterface::GetDefaultAcquireInterface(void)

{
                    /* 0x51d50  77  ?GetDefaultAcquireInterface@GDefaultAcquireInterface@@SAPAV1@XZ
                        */
  if ((DAT_01ea506c & 1) == 0) {
    DAT_01ea506c = DAT_01ea506c | 1;
    FUN_00451d90((undefined4 *)&DAT_01ea5068);
    FUN_012375cb(FUN_017da850);
  }
  return (GDefaultAcquireInterface *)&DAT_01ea5068;
}




/* === Additional global functions for GDefaultAcquireInterface (1 functions) === */

/* Also in vtable: GDefaultAcquireInterface (slot 0) */

undefined4 * __thiscall GDefaultAcquireInterface__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01678448;
  local_c = ExceptionList;
  if ((param_1 & 2) == 0) {
    ExceptionList = &local_c;
    *(undefined ***)this = GDefaultAcquireInterface::vftable;
    local_4 = 0xffffffff;
    *(undefined ***)this = GAcquireInterface::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    ExceptionList = pvVar1;
    return this;
  }
  ExceptionList = &local_c;
  _eh_vector_destructor_iterator_
            (this,4,*(int *)((int)this + -4),GDefaultAcquireInterface::~GDefaultAcquireInterface);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  ExceptionList = local_c;
  return (undefined4 *)((int)this + -4);
}




/* ========== GDelegatedFile.c ========== */

/*
 * SGW.exe - GDelegatedFile (1 functions)
 */

GRefCountBaseImpl * __thiscall GDelegatedFile__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GEvent.c ========== */

/*
 * SGW.exe - GEvent (11 functions)
 */


/* public: void __thiscall GEvent::`default constructor closure'(void) */

void __thiscall GEvent::_default_constructor_closure_(GEvent *this)

{
                    /* 0x1ced0  34  ??_FGEvent@@QAEXXZ */
  GEvent(this,false,false);
  return;
}




/* public: __thiscall GEvent::GEvent(bool,bool) */

GEvent * __thiscall GEvent::GEvent(GEvent *this,bool param_1,bool param_2)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x525f0  3  ??0GEvent@@QAE@_N0@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167ab3e;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004514d0(this,param_2);
  local_8 = 0;
  FUN_00451db0((undefined4 *)(this + 0x14));
  local_8._0_1_ = 1;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  GMutex::GMutex((GMutex *)(this + 0x1c),true,false);
  local_8 = CONCAT31(local_8._1_3_,2);
  GWaitCondition::GWaitCondition((GWaitCondition *)(this + 0x38));
  this[0x18] = (GEvent)param_1;
  this[0x19] = (GEvent)0x0;
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GEvent::~GEvent(void) */

void __thiscall GEvent::~GEvent(GEvent *this)

{
  GAcquireInterface *local_18;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
                    /* 0x52710  21  ??1GEvent@@UAE@XZ */
  puStack_c = &LAB_0167ab9a;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  local_8 = 2;
  GWaitCondition::~GWaitCondition((GWaitCondition *)(this + 0x38));
  local_8._0_1_ = 1;
  GMutex::~GMutex((GMutex *)(this + 0x1c));
  local_8 = (uint)local_8._1_3_ << 8;
  if (this == (GEvent *)0x0) {
    local_18 = (GAcquireInterface *)0x0;
  }
  else {
    local_18 = (GAcquireInterface *)(this + 0x14);
  }
  GAcquireInterface::~GAcquireInterface(local_18);
  local_8 = 0xffffffff;
  FUN_00451650((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* public: bool __thiscall GEvent::Wait(unsigned int) */

bool __thiscall GEvent::Wait(GEvent *this,uint param_1)

{
  GEvent GVar1;
  undefined4 local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x527b0  169  ?Wait@GEvent@@QAE_NI@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167abc8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004528a0(&local_14,this + 0x1c);
  local_8 = 0;
  if (param_1 != 0) {
    if (param_1 == 0xffffffff) {
      while (this[0x18] == (GEvent)0x0) {
        GWaitCondition::Wait((GWaitCondition *)(this + 0x38),(GMutex *)(this + 0x1c),0xffffffff);
      }
    }
    else if (this[0x18] == (GEvent)0x0) {
      GWaitCondition::Wait((GWaitCondition *)(this + 0x38),(GMutex *)(this + 0x1c),param_1);
    }
  }
  GVar1 = this[0x18];
  if (this[0x19] != (GEvent)0x0) {
    this[0x19] = (GEvent)0x0;
    this[0x18] = (GEvent)0x0;
  }
  local_8 = 0xffffffff;
  FUN_004528d0(&local_14);
  ExceptionList = local_10;
  return (bool)GVar1;
}




/* public: void __thiscall GEvent::SetEvent(void) */

void __thiscall GEvent::SetEvent(GEvent *this)

{
  cancellation_token_source local_14 [4];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x528f0  134  ?SetEvent@GEvent@@QAEXXZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167abf8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  GMutex::Lock((GMutex *)(this + 0x1c));
  this[0x18] = (GEvent)0x1;
  this[0x19] = (GEvent)0x0;
  GWaitCondition::NotifyAll((GWaitCondition *)(this + 0x38));
  FUN_014babb0((undefined4 *)local_14);
  local_8 = 0;
  FUN_004529c0(this,local_14);
  GMutex::Unlock((GMutex *)(this + 0x1c));
  FUN_00452990((undefined4 *)local_14);
  local_8 = 0xffffffff;
  FUN_004529f0(local_14);
  ExceptionList = local_10;
  return;
}




/* public: void __thiscall GEvent::ResetEvent(void) */

void __thiscall GEvent::ResetEvent(GEvent *this)

{
  undefined4 local_8;
  
                    /* 0x52a10  128  ?ResetEvent@GEvent@@QAEXXZ */
  FUN_004528a0(&local_8,this + 0x1c);
  this[0x18] = (GEvent)0x0;
  this[0x19] = (GEvent)0x0;
  FUN_004528d0(&local_8);
  return;
}




/* public: void __thiscall GEvent::PulseEvent(void) */

void __thiscall GEvent::PulseEvent(GEvent *this)

{
  cancellation_token_source local_14 [4];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x52a50  122  ?PulseEvent@GEvent@@QAEXXZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167abf8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  GMutex::Lock((GMutex *)(this + 0x1c));
  this[0x18] = (GEvent)0x1;
  this[0x19] = (GEvent)0x1;
  GWaitCondition::Notify((GWaitCondition *)(this + 0x38));
  FUN_014babb0((undefined4 *)local_14);
  local_8 = 0;
  FUN_004529c0(this,local_14);
  GMutex::Unlock((GMutex *)(this + 0x1c));
  FUN_00452990((undefined4 *)local_14);
  local_8 = 0xffffffff;
  FUN_004529f0(local_14);
  ExceptionList = local_10;
  return;
}




/* public: virtual bool __thiscall GEvent::CanAcquire(void) */

bool __thiscall GEvent::CanAcquire(GEvent *this)

{
  undefined1 uVar1;
  
                    /* 0x52af0  50  ?CanAcquire@GEvent@@UAE_NXZ
                       0x52af0  156  ?TryAcquire@GEvent@@UAE_NXZ */
  uVar1 = (**(code **)(*(int *)(this + -0x14) + 4))();
  return (bool)uVar1;
}




/* public: virtual bool __thiscall GEvent::TryAcquireCommit(void) */

bool __thiscall GEvent::TryAcquireCommit(GEvent *this)

{
  undefined4 local_8;
  
                    /* 0x52b10  164  ?TryAcquireCommit@GEvent@@UAE_NXZ */
  FUN_004528a0(&local_8,this + 8);
  if (this[5] != (GEvent)0x0) {
    this[5] = (GEvent)0x0;
    this[4] = (GEvent)0x0;
  }
  FUN_004528d0(&local_8);
  return true;
}




/* public: virtual class GAcquireInterface * __thiscall GEvent::GetAcquireInterface(void)
   Also in vtable: GSemaphore (slot 2)
   Also in vtable: GSemaphoreWaitableIncrement (slot 2)
   Also in vtable: GThread (slot 2) */

GAcquireInterface * __thiscall GEvent::GetAcquireInterface(GEvent *this)

{
  GAcquireInterface *local_c;
  
                    /* 0x54800  72  ?GetAcquireInterface@GEvent@@UAEPAVGAcquireInterface@@XZ
                       0x54800  74  ?GetAcquireInterface@GSemaphore@@UAEPAVGAcquireInterface@@XZ */
  if (this == (GEvent *)0x0) {
    local_c = (GAcquireInterface *)0x0;
  }
  else {
    local_c = (GAcquireInterface *)(this + 0x14);
  }
  return local_c;
}




/* public: virtual bool __thiscall GEvent::IsSignaled(void)const  */

bool __thiscall GEvent::IsSignaled(GEvent *this)

{
                    /* 0x10e73f0  99  ?IsSignaled@GEvent@@UBE_NXZ */
  return (bool)this[0x18];
}




/* === Additional global functions for GEvent (1 functions) === */

/* Also in vtable: GEvent (slot 0) */

GEvent * __thiscall GEvent__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GEvent::~GEvent(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_(this,0x3c,*(int *)((int)this + -4),GEvent::~GEvent);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== GFILEFile.c ========== */

/*
 * SGW.exe - GFILEFile (1 functions)
 */

GRefCountBaseImpl * __thiscall GFILEFile__vfunc_0(void *this,uint param_1)

{
  FUN_013cd530(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GFile.c ========== */

/*
 * SGW.exe - GFile (1 functions)
 */

/* Also in vtable: GFile (slot 0) */

GRefCountBaseImpl * __thiscall GFile__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016daa70;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== GFx_Scaleform.c ========== */

/*
 * SGW.exe - GFx_Scaleform (1 functions)
 */


/* public: virtual __thiscall GFxLoader::~GFxLoader(void) */

void __thiscall GFxLoader::~GFxLoader(GFxLoader *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfcd000  22  ??1GFxLoader@@UAE@XZ */
  puStack_c = &LAB_0177e1f8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  local_8 = 0;
  if (*(int *)(this + 4) != 0) {
    FUN_00454b00(*(int *)(this + 4));
  }
  if (*(int *)(this + 8) != 0) {
    FUN_00454b00(*(int *)(this + 8));
  }
  local_8 = 0xffffffff;
  FUN_013cce30((undefined4 *)this);
  ExceptionList = local_10;
  return;
}




/* === Additional global functions for GFx_Scaleform (111 functions) === */

/* Also in vtable: GFxState (slot 0) */

GRefCountBaseImpl * __thiscall GFxState__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0168e200;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = GFxState::vftable;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}


/* Also in vtable: GFxFSCommandHandler (slot 0) */

GRefCountBaseImpl * __thiscall GFxFSCommandHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0168e228;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = GFxState::vftable;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}


/* Library Function - Multiple Matches With Different Base Names
    public: virtual __thiscall Concurrency::details::_CancellationTokenCallback<class
   <lambda_61f7764e5b8087545c74b0c2f4f68b12> >::~_CancellationTokenCallback<class
   <lambda_61f7764e5b8087545c74b0c2f4f68b12> >(void)
    public: virtual __thiscall Concurrency::details::_CancellationTokenCallback<class
   <lambda_a164222bfc9743191016d5f980c35833> >::~_CancellationTokenCallback<class
   <lambda_a164222bfc9743191016d5f980c35833> >(void)
    public: virtual __thiscall std::_Ref_count_del_alloc<class __ExceptionPtr,void (__cdecl*)(class
   __ExceptionPtr *),class _DebugMallocator<int> >::~_Ref_count_del_alloc<class __ExceptionPtr,void
   (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >(void)
    public: virtual __thiscall std::tr1::_Ref_count_del_alloc<class __ExceptionPtr,void
   (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >::~_Ref_count_del_alloc<class
   __ExceptionPtr,void (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >(void)
     7 names - too many to list
   
   Libraries: Visual Studio 2005, Visual Studio 2008, Visual Studio 2010, Visual Studio 2012 */

void __fastcall
FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>
          (GRefCountBaseImpl *param_1)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  puStack_c = &LAB_0177dd28;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  local_8 = 0;
  GFxSprite__unknown_013d9ff0((int *)(param_1 + 0x10));
  local_8 = 0xffffffff;
  FUN_00aff230(param_1);
  ExceptionList = local_10;
  return;
}


GRefCountBaseImpl * __thiscall GFxResourceLib__vfunc_0(void *this,uint param_1)

{
  FUN_013c9e40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxResourceLib_ResourceSlot__vfunc_0(void *this,uint param_1)

{
  FUN_013ca050(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxResourceWeakLib__vfunc_0(void *this,uint param_1)

{
  FUN_013ca4d0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


/* [VTable] GFxFileOpener virtual function #1
   VTable: vtable_GFxFileOpener at 01960c10 */

undefined4 GFxFileOpener__vfunc_1(char *param_1)

{
  bool bVar1;
  GSysFile *this;
  undefined4 *puVar2;
  GFile *pGVar3;
  undefined4 local_40;
  undefined4 local_34;
  int local_20;
  GBufferedFile *local_1c;
  undefined4 local_18;
  undefined4 local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e0df;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  bVar1 = false;
  local_1c = (GBufferedFile *)GFxSprite__unknown_00453600(0x2c);
  local_8 = 0;
  if (local_1c == (GBufferedFile *)0x0) {
    local_40 = 0;
  }
  else {
    this = (GSysFile *)GFxSprite__unknown_00453600(0x14);
    local_8._0_1_ = 1;
    if (this == (GSysFile *)0x0) {
      local_34 = 0;
    }
    else {
      local_34 = GSysFile::GSysFile(this,param_1,1,0x1b6);
    }
    local_8._0_1_ = 0;
    puVar2 = (undefined4 *)FUN_00aef690(&local_20,local_34);
    local_8 = CONCAT31(local_8._1_3_,2);
    bVar1 = true;
    pGVar3 = (GFile *)GImageInfoBaseImpl__unknown_00aef390(puVar2);
    local_40 = GBufferedFile::GBufferedFile(local_1c,pGVar3);
  }
  local_18 = local_40;
  local_14 = local_40;
  local_8 = 0xffffffff;
  if (bVar1) {
    FUN_00aef370(&local_20);
  }
  ExceptionList = local_10;
  return local_14;
}


/* [VTable] GFxImageCreator virtual function #1
   VTable: vtable_GFxImageCreator at 01960bf8 */

GRefCountBaseImpl * __thiscall GFxImageCreator__vfunc_1(void *this,int *param_1)

{
  char cVar1;
  uint uVar2;
  int iVar3;
  undefined4 uVar4;
  int *piVar5;
  GRefCountBaseImpl *pGVar6;
  void *this_00;
  int iVar7;
  uint uVar8;
  GRefCountBaseImpl GVar9;
  GRefCountBaseImpl *local_9c;
  int local_94;
  GRefCountBaseImpl *local_88;
  GRefCountBaseImpl *local_84;
  GRefCountBaseImpl *local_80;
  GRefCountBaseImpl *local_7c;
  int local_24;
  int local_20;
  uint local_1c;
  int local_18;
  uint local_14;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e17f;
  local_10 = ExceptionList;
  uVar2 = DAT_01e7f9a0 ^ (uint)&stack0xfffffffc;
  ExceptionList = &local_10;
  FUN_01441890(&local_18);
  local_8 = 0;
  local_1c = 0;
  local_14 = 0;
  if (*param_1 == 1) {
    GFxTextDocView__unknown_013e1050(&local_18,param_1[3]);
    iVar3 = GFxSprite__unknown_014b3780(&local_18);
    local_1c = *(uint *)(iVar3 + 0x14);
    iVar3 = GFxSprite__unknown_014b3780(&local_18);
    local_14 = *(uint *)(iVar3 + 0x18);
  }
  else {
    if (*param_1 != 2) {
      pGVar6 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x28);
      local_8 = CONCAT31(local_8._1_3_,2);
      if (pGVar6 == (GRefCountBaseImpl *)0x0) {
        local_7c = (GRefCountBaseImpl *)0x0;
      }
      else {
        local_7c = FUN_013cf7e0(pGVar6,0,(GRefCountBaseImpl)0x0);
      }
      local_8 = 0xffffffff;
      GFxSprite__unknown_013d9ff0(&local_18);
      ExceptionList = local_10;
      return local_7c;
    }
    iVar3 = FID__unknown_013ca1b0((int *)(param_1[3] + 0x18));
    uVar4 = (**(code **)(*(int *)param_1[4] + 4))(iVar3,uVar2);
    GFxSprite__unknown_01407130(&local_24,uVar4);
    local_8._0_1_ = 1;
    iVar3 = GFxSprite__unknown_014b3780(&local_24);
    if (iVar3 == 0) {
      local_8 = (uint)local_8._1_3_ << 8;
      GFxSprite__unknown_013d9ff0(&local_24);
      local_8 = 0xffffffff;
      GFxSprite__unknown_013d9ff0(&local_18);
      ExceptionList = local_10;
      return (GRefCountBaseImpl *)0x0;
    }
    iVar3 = param_1[1];
    iVar7 = *(int *)(param_1[3] + 0x10);
    piVar5 = (int *)GFxSprite__unknown_014b3780(&local_24);
    iVar3 = FUN_013ccb90(piVar5,iVar7,iVar3);
    GFxSprite__unknown_014dedc0(&local_18,iVar3);
    iVar3 = GFxSprite__unknown_014b3780(&local_18);
    if (iVar3 == 0) {
      local_8 = (uint)local_8._1_3_ << 8;
      GFxSprite__unknown_013d9ff0(&local_24);
      local_8 = 0xffffffff;
      GFxSprite__unknown_013d9ff0(&local_18);
      ExceptionList = local_10;
      return (GRefCountBaseImpl *)0x0;
    }
    local_1c = (uint)*(ushort *)(param_1[3] + 0x1c);
    local_14 = (uint)*(ushort *)(param_1[3] + 0x1e);
    local_8 = (uint)local_8._1_3_ << 8;
    GFxSprite__unknown_013d9ff0(&local_24);
  }
  cVar1 = FUN_013ccb70((int)this);
  if (cVar1 != '\0') {
    pGVar6 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x28);
    local_8 = CONCAT31(local_8._1_3_,3);
    if (pGVar6 == (GRefCountBaseImpl *)0x0) {
      local_80 = (GRefCountBaseImpl *)0x0;
    }
    else {
      GVar9 = (GRefCountBaseImpl)0x0;
      uVar2 = local_1c;
      uVar8 = local_14;
      iVar3 = GFxSprite__unknown_014b3780(&local_18);
      local_80 = GFxImageCreator__unknown_013cf8a0(pGVar6,iVar3,uVar2,uVar8,GVar9);
    }
    local_8 = 0xffffffff;
    GFxSprite__unknown_013d9ff0(&local_18);
    ExceptionList = local_10;
    return local_80;
  }
  if ((param_1[5] != 0) && (iVar3 = FUN_01485760(param_1[5]), iVar3 != 0)) {
    uVar2 = GFxTextDocView__unknown_01485570(param_1[5]);
    if ((uVar2 & 0x10000) != 0) {
      pGVar6 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x28);
      local_8 = CONCAT31(local_8._1_3_,4);
      if (pGVar6 == (GRefCountBaseImpl *)0x0) {
        local_84 = (GRefCountBaseImpl *)0x0;
      }
      else {
        GVar9 = (GRefCountBaseImpl)0x0;
        uVar2 = local_1c;
        uVar8 = local_14;
        iVar3 = GFxSprite__unknown_014b3780(&local_18);
        local_84 = GFxImageCreator__unknown_013cf8a0(pGVar6,iVar3,uVar2,uVar8,GVar9);
      }
      local_8 = 0xffffffff;
      GFxSprite__unknown_013d9ff0(&local_18);
      ExceptionList = local_10;
      return local_84;
    }
    if (((char)param_1[6] != '\0') &&
       (uVar2 = GFxTextDocView__unknown_01485570(param_1[5]), (uVar2 & 0x100000) == 0)) {
      pGVar6 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x28);
      local_8 = CONCAT31(local_8._1_3_,5);
      if (pGVar6 == (GRefCountBaseImpl *)0x0) {
        local_88 = (GRefCountBaseImpl *)0x0;
      }
      else {
        GVar9 = (GRefCountBaseImpl)0x1;
        uVar2 = local_1c;
        uVar8 = local_14;
        iVar3 = GFxSprite__unknown_014b3780(&local_18);
        local_88 = GFxImageCreator__unknown_013cf8a0(pGVar6,iVar3,uVar2,uVar8,GVar9);
      }
      local_8 = 0xffffffff;
      GFxSprite__unknown_013d9ff0(&local_18);
      ExceptionList = local_10;
      return local_88;
    }
    piVar5 = (int *)FUN_01485760(param_1[5]);
    uVar4 = (**(code **)(*piVar5 + 8))();
    GFxSprite__unknown_01407130(&local_20,uVar4);
    local_8._0_1_ = 6;
    iVar3 = GImageInfoBaseImpl__unknown_00aef390(&local_20);
    if (iVar3 != 0) {
      local_94 = GFxSprite__unknown_014b3780(&local_18);
      if (local_94 == 0) {
        local_94 = 0;
      }
      else {
        local_94 = local_94 + 0x10;
      }
      piVar5 = (int *)GImageInfoBaseImpl__unknown_00aef390(&local_20);
      cVar1 = (**(code **)(*piVar5 + 8))(local_94,local_1c,local_14);
      if (cVar1 != '\0') {
        this_00 = (void *)GFxSprite__unknown_00453600(0x28);
        local_8._0_1_ = 7;
        if (this_00 == (void *)0x0) {
          local_9c = (GRefCountBaseImpl *)0x0;
        }
        else {
          uVar2 = local_1c;
          uVar8 = local_14;
          piVar5 = (int *)GImageInfoBaseImpl__unknown_00aef390(&local_20);
          local_9c = (GRefCountBaseImpl *)FUN_013cf930(this_00,piVar5,uVar2,uVar8);
        }
        local_8 = (uint)local_8._1_3_ << 8;
        FUN_00af0600(&local_20);
        local_8 = 0xffffffff;
        GFxSprite__unknown_013d9ff0(&local_18);
        ExceptionList = local_10;
        return local_9c;
      }
    }
    local_8 = (uint)local_8._1_3_ << 8;
    FUN_00af0600(&local_20);
    local_8 = 0xffffffff;
    GFxSprite__unknown_013d9ff0(&local_18);
    ExceptionList = local_10;
    return (GRefCountBaseImpl *)0x0;
  }
  local_8 = 0xffffffff;
  GFxSprite__unknown_013d9ff0(&local_18);
  ExceptionList = local_10;
  return (GRefCountBaseImpl *)0x0;
}


GRefCountBaseImpl * __thiscall GFxParseControl__vfunc_0(void *this,uint param_1)

{
  FUN_013cce10(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


int __fastcall GFxLoader__vfunc_0(int param_1)

{
  undefined4 local_c;
  
  if (*(int *)(param_1 + 4) == 0) {
    local_c = 0;
  }
  else {
    local_c = *(int *)(param_1 + 4) + 0x10;
  }
  return local_c;
}


GRefCountBaseImpl * __thiscall GFxRenderConfig__vfunc_0(void *this,uint param_1)

{
  ~_Ref_count_del_alloc<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


/* Library Function - Multiple Matches With Same Base Name
    public: virtual __thiscall std::_Ref_count_del_alloc<class __ExceptionPtr,void (__cdecl*)(class
   __ExceptionPtr *),class _DebugMallocator<int> >::~_Ref_count_del_alloc<class __ExceptionPtr,void
   (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >(void)
    public: virtual __thiscall std::tr1::_Ref_count_del_alloc<class __ExceptionPtr,void
   (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >::~_Ref_count_del_alloc<class
   __ExceptionPtr,void (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >(void)
   
   Libraries: Visual Studio 2010 Debug, Visual Studio 2012 Debug */

void __fastcall ~_Ref_count_del_alloc<>(GRefCountBaseImpl *param_1)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  puStack_c = &LAB_01780808;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  local_8 = 0;
  GFxSprite__unknown_013d9ff0((int *)(param_1 + 0x14));
  local_8 = 0xffffffff;
  FUN_00569470(param_1);
  ExceptionList = local_10;
  return;
}


GRefCountBaseImpl * __thiscall GFxTask__vfunc_0(void *this,uint param_1)

{
  FUN_013d6330(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxLoaderTask__vfunc_0(void *this,uint param_1)

{
  FUN_013d63b0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxLog__vfunc_0(void *this,uint param_1)

{
  FUN_013d68f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxSharedStateImpl__vfunc_0(void *this,uint param_1)

{
  FUN_013d6d90(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxLoaderImpl__vfunc_0(void *this,uint param_1)

{
  FUN_013d6f80(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxResource__vfunc_0(void *this,uint param_1)

{
  FUN_013d8f80(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCRibbonCustomizeCategory * __thiscall GFxImageResource__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonCustomizeCategory::~CMFCRibbonCustomizeCategory(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CBitmapButton * __thiscall GFxMovieImageLoadTask__vfunc_0(void *this,uint param_1)

{
  CBitmapButton::~CBitmapButton(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxResourceFileInfo__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxImageFileInfo__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCToolBarsListCheckBox * __thiscall GFxTextureGlyph__vfunc_0(void *this,uint param_1)

{
  CMFCToolBarsListCheckBox::~CMFCToolBarsListCheckBox(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XImage * __thiscall GFxTextureGlyphData__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XImage::~XImage(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxInitImportActions__vfunc_0(void *this,uint param_1)

{
  FUN_013df620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Library Function - Multiple Matches With Different Base Names
    public: __thiscall Concurrency::details::_Async_send_queue<class Concurrency::message<unsigned
   int> >::~_Async_send_queue<class Concurrency::message<unsigned int> >(void)
    public: __thiscall Concurrency::details::_Async_send_queue<class Concurrency::message<enum
   Concurrency::agent_status> >::~_Async_send_queue<class Concurrency::message<enum
   Concurrency::agent_status> >(void)
    public: virtual __thiscall Concurrency::details::_CancellationTokenCallback<class
   <lambda_61f7764e5b8087545c74b0c2f4f68b12> >::~_CancellationTokenCallback<class
   <lambda_61f7764e5b8087545c74b0c2f4f68b12> >(void)
    public: virtual __thiscall Concurrency::details::_CancellationTokenCallback<class
   <lambda_a164222bfc9743191016d5f980c35833> >::~_CancellationTokenCallback<class
   <lambda_a164222bfc9743191016d5f980c35833> >(void)
   
   Libraries: Visual Studio 2010 Debug, Visual Studio 2010 Release, Visual Studio 2012 Debug, Visual
   Studio 2012 Release */

void __fastcall
FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>
          (int *param_1)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  puStack_c = &LAB_017880e8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  local_8 = 0;
  GFxSprite__unknown_013df870((byte *)(param_1 + 3));
  local_8 = 0xffffffff;
  FID__unknown_013df830(param_1);
  ExceptionList = local_10;
  return;
}


CRecentDockSiteInfo * __thiscall GFxStream__vfunc_0(void *this,uint param_1)

{
  CRecentDockSiteInfo::~CRecentDockSiteInfo(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxCharacterDef__vfunc_0(void *this,uint param_1)

{
  FUN_013e5510(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxEditTextCharacterDef__vfunc_0(void *this,uint param_1)

{
  FUN_013e55e0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxTextFormat__vfunc_0(void *this,uint param_1)

{
  GFxTextDocView__unknown_013e7fc0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxTextParagraphFormat__vfunc_0(void *this,uint param_1)

{
  ~_Mpunct<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxEditTextCharacter__vfunc_0(void *this,uint param_1)

{
  FUN_013f1730(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XImage * __thiscall GFxMovieDefImplKey__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XImage::~XImage(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxTextClipboard__vfunc_0(void *this,uint param_1)

{
  CMFCFilterChunkValueImpl::~CMFCFilterChunkValueImpl(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxTextKeyMap__vfunc_0(void *this,uint param_1)

{
  ~_Ref_count_del_alloc<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxFontCacheManager__vfunc_0(void *this,uint param_1)

{
  FUN_013fbcc0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxFontCacheManagerImpl_RenEventHandler__vfunc_0(void *this,uint param_1)

{
  FUN_013fc290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxFontResource_DisposeHandler__vfunc_0(void *this,uint param_1)

{
  FUN_013fc310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxFontCacheManagerImpl_FontDisposeHandler__vfunc_0(void *this,uint param_1)

{
  FUN_013fc2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxLoadStates__vfunc_0(void *this,uint param_1)

{
  FUN_014049f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieDefBindStates__vfunc_0(void *this,uint param_1)

{
  FUN_01404910(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxLoadProcess__vfunc_0(void *this,uint param_1)

{
  FUN_014052a0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxResourceData_DataInterface__vfunc_0(void *this,uint param_1)

{
  FUN_014064d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxMovieDataDef__vfunc_0(void *this,uint param_1)

{
  FUN_01407910(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxTimelineDef__vfunc_0(void *this,uint param_1)

{
  FUN_014078c0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieDataDef_LoadTaskData__vfunc_0(void *this,uint param_1)

{
  FUN_014081f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieDataDef_LoadTaskDataBase__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CBitmapButton * __thiscall GFxMovieDataDefFileKeyData__vfunc_0(void *this,uint param_1)

{
  CBitmapButton::~CBitmapButton(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxMovieDefImpl__vfunc_0(void *this,uint param_1)

{
  FUN_0140a420(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxMovieDef__vfunc_0(void *this,uint param_1)

{
  FUN_0140a380(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieBindProcess__vfunc_0(void *this,uint param_1)

{
  FUN_0140a650(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieDefImpl_BindTaskData__vfunc_0(void *this,uint param_1)

{
  FUN_0140b7f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxResourceKey_KeyInterface__vfunc_0(void *this,uint param_1)

{
  FUN_014119f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxActionLogger__vfunc_0(void *this,uint param_1)

{
  FUN_01411e10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Library Function - Multiple Matches With Different Base Names
    public: virtual __thiscall Concurrency::details::_CancellationTokenCallback<class
   <lambda_61f7764e5b8087545c74b0c2f4f68b12> >::~_CancellationTokenCallback<class
   <lambda_61f7764e5b8087545c74b0c2f4f68b12> >(void)
    public: virtual __thiscall Concurrency::details::_CancellationTokenCallback<class
   <lambda_a164222bfc9743191016d5f980c35833> >::~_CancellationTokenCallback<class
   <lambda_a164222bfc9743191016d5f980c35833> >(void)
    public: virtual __thiscall std::_Ref_count_del_alloc<class __ExceptionPtr,void (__cdecl*)(class
   __ExceptionPtr *),class _DebugMallocator<int> >::~_Ref_count_del_alloc<class __ExceptionPtr,void
   (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >(void)
    public: virtual __thiscall std::tr1::_Ref_count_del_alloc<class __ExceptionPtr,void
   (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >::~_Ref_count_del_alloc<class
   __ExceptionPtr,void (__cdecl*)(class __ExceptionPtr *),class _DebugMallocator<int> >(void)
     7 names - too many to list
   
   Libraries: Visual Studio 2005, Visual Studio 2008, Visual Studio 2010, Visual Studio 2012 */

void __fastcall
FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>
          (cancellation_token_source *param_1)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  puStack_c = &LAB_01784748;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  local_8 = 0;
  GFxSprite__unknown_013df870((byte *)(param_1 + 4));
  local_8 = 0xffffffff;
  Concurrency::cancellation_token_source::cancel(param_1);
  ExceptionList = local_10;
  return;
}


/* Library Function - Multiple Matches With Same Base Name
    public: __thiscall
   <lambda_399686a4a5619bf4b18c998976ba8b81>::<lambda_399686a4a5619bf4b18c998976ba8b81>(class
   <lambda_399686a4a5619bf4b18c998976ba8b81> const &)
    public: __thiscall
   <lambda_9ab38a79c03e1e04423ad6818173354b>::<lambda_9ab38a79c03e1e04423ad6818173354b>(class
   <lambda_9ab38a79c03e1e04423ad6818173354b> const &)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

undefined4 * __thiscall <>(void *this,undefined4 *param_1)

{
  *(undefined4 *)this = *param_1;
  GFxTextDocView__unknown_01406fe0((void *)((int)this + 4),param_1 + 1);
  return this;
}


undefined4 * __thiscall GFxStaticTextCharacterDef__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxKeyboardState_IListener__vfunc_0(void *this,uint param_1)

{
  FUN_01430ea0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxStaticTextCharacter__vfunc_0(void *this,uint param_1)

{
  FUN_014317b0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMoviePreloadTask__vfunc_0(void *this,uint param_1)

{
  FID__unknown_01431e90(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxLoadQueueEntryMT__vfunc_0(void *this,uint param_1)

{
  ~basic_streambuf<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxLoadQueueEntryMT_LoadMovie__vfunc_0(void *this,uint param_1)

{
  FUN_01432540(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxLoadVarsTask__vfunc_0(void *this,uint param_1)

{
  FID__unknown_01432f00(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XQAT * __thiscall GFxLoadQueueEntryMT_LoadVars__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XQAT::~XQAT(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieRoot__vfunc_0(void *this,uint param_1)

{
  FUN_01433e60(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxMovieView__vfunc_0(void *this,uint param_1)

{
  FUN_01433d40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxPlaceObject2__vfunc_0(void *this,uint param_1)

{
  FUN_01440170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Library Function - Multiple Matches With Same Base Name
    public: __thiscall
   <lambda_399686a4a5619bf4b18c998976ba8b81>::<lambda_399686a4a5619bf4b18c998976ba8b81>(class
   <lambda_399686a4a5619bf4b18c998976ba8b81> const &)
    public: __thiscall
   <lambda_9ab38a79c03e1e04423ad6818173354b>::<lambda_9ab38a79c03e1e04423ad6818173354b>(class
   <lambda_9ab38a79c03e1e04423ad6818173354b> const &)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

undefined4 * __thiscall <>(void *this,undefined4 *param_1)

{
  *(undefined4 *)this = *param_1;
  GFxSprite__unknown_014908e0((void *)((int)this + 4),param_1 + 1);
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxShapeCharacterDef__vfunc_0(void *this,uint param_1)

{
  CMFCFilterChunkValueImpl::~CMFCFilterChunkValueImpl(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxShapeNoStylesDef__vfunc_0(void *this,uint param_1)

{
  FID_conflict___CancellationTokenCallback<class_<lambda_61f7764e5b8087545c74b0c2f4f68b12>_>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxConstShapeNoStylesDef__vfunc_0(void *this,uint param_1)

{
  FUN_01448e80(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxConstShapeWithStylesDef__vfunc_0(void *this,uint param_1)

{
  FUN_01449010(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxShapeWithStylesDef__vfunc_0(void *this,uint param_1)

{
  FUN_01449770(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CMFCFilterChunkValueImpl * __thiscall GFxMorphCharacterDef__vfunc_0(void *this,uint param_1)

{
  FUN_0144ead0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxFont__vfunc_0(void *this,uint param_1)

{
  FUN_01451d10(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxFontData__vfunc_0(void *this,uint param_1)

{
  FUN_01451e60(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxFontResource__vfunc_0(void *this,uint param_1)

{
  FUN_01453a70(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XImage * __thiscall GFxSystemFontResourceKey__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XImage::~XImage(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxSpriteDef__vfunc_0(void *this,uint param_1)

{
  FUN_01456a40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxTimelineIODef__vfunc_0(void *this,uint param_1)

{
  FUN_014569f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxSprite__vfunc_0(void *this,uint param_1)

{
  FUN_01457a30(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxButtonCharacterDef__vfunc_0(void *this,uint param_1)

{
  FUN_014637e0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxButtonCharacter__vfunc_0(void *this,uint param_1)

{
  FUN_014662b0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxCharacter__vfunc_0(void *this,uint param_1)

{
  FUN_01474fe0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxASCharacter__vfunc_0(void *this,uint param_1)

{
  FUN_01475a00(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxGenericCharacter__vfunc_0(void *this,uint param_1)

{
  FUN_0147a2e0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CBitmapButton * __thiscall GFxStyledText__vfunc_0(void *this,uint param_1)

{
  CBitmapButton::~CBitmapButton(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XQAT * __thiscall GFxTextAllocator__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XQAT::~XQAT(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


CBitmapButton * __thiscall GFxTextDocView_DocumentText__vfunc_0(void *this,uint param_1)

{
  FUN_01484010(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxTextDocView__vfunc_0(void *this,uint param_1)

{
  FUN_01484060(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxDrawingContext__vfunc_0(void *this,uint param_1)

{
  FUN_014841f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxTextEditorKit__vfunc_0(void *this,uint param_1)

{
  FUN_0148cec0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XQAT * __thiscall GFxTextCompositionString__vfunc_0(void *this,uint param_1)

{
  CMFCRibbonInfo::XQAT::~XQAT(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


XImage * __thiscall GFxTextHTMLImageTagDesc__vfunc_0(void *this,uint param_1)

{
  FUN_014943d0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


/* Library Function - Multiple Matches With Same Base Name
    public: __thiscall
   <lambda_399686a4a5619bf4b18c998976ba8b81>::<lambda_399686a4a5619bf4b18c998976ba8b81>(class
   <lambda_399686a4a5619bf4b18c998976ba8b81> const &)
    public: __thiscall
   <lambda_9ab38a79c03e1e04423ad6818173354b>::<lambda_9ab38a79c03e1e04423ad6818173354b>(class
   <lambda_9ab38a79c03e1e04423ad6818173354b> const &)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

undefined4 * __thiscall <>(void *this,undefined4 *param_1)

{
  *(undefined4 *)this = *param_1;
  GFxTextDocView__unknown_013f9470((void *)((int)this + 4),param_1 + 1);
  return this;
}


undefined4 * __thiscall GFxASMouseListener__vfunc_0(void *this,uint param_1)

{
  FUN_0149e3b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxFontManager__vfunc_0(void *this,uint param_1)

{
  FUN_014ad8e0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


/* Also in vtable: VGWaitable___GRefCountBase (slot 0) */

GRefCountBaseImpl * __thiscall
GFxKeyboardState_VIListener___GRefCountBase__vfunc_0(void *this,uint param_1)

{
  FUN_014048f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxGlyphRasterCache_TextureEventHandler__vfunc_0(void *this,uint param_1)

{
  FUN_014b0ad0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxImageFileInfoKeyData__vfunc_0(void *this,uint param_1)

{
  FUN_014b9180(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxGradientImageResourceKey__vfunc_0(void *this,uint param_1)

{
  FUN_014119d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxKeyboardState__vfunc_0(void *this,uint param_1)

{
  FUN_014bc0d0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


/* Also in vtable: GAllocator (slot 4) */

undefined4 GFxSharedState__vfunc_0(void)

{
  return 0;
}


GRefCountBaseImpl * __thiscall GFxScale9GridInfo__vfunc_0(void *this,uint param_1)

{
  FUN_014df480(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxMeshSet__vfunc_0(void *this,uint param_1)

{
  FUN_014e3c60(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


GRefCountBaseImpl * __thiscall GFxGradientData__vfunc_0(void *this,uint param_1)

{
  FUN_014e6320(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


undefined4 * __thiscall GFxLineStyle__vfunc_0(void *this,uint param_1)

{
  FUN_014e8ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxFontResourceCreator__vfunc_0(void *this,uint param_1)

{
  FUN_014064b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxVertexInterface__vfunc_0(void *this,uint param_1)

{
  FUN_014ee4a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall GFxVertexInterface_XY16i__vfunc_0(void *this,uint param_1)

{
  FUN_014ee7f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Library Function - Multiple Matches With Same Base Name
    public: __thiscall
   <lambda_399686a4a5619bf4b18c998976ba8b81>::~<lambda_399686a4a5619bf4b18c998976ba8b81>(void)
    public: __thiscall
   <lambda_9ab38a79c03e1e04423ad6818173354b>::~<lambda_9ab38a79c03e1e04423ad6818173354b>(void)
   
   Libraries: Visual Studio 2012 Debug, Visual Studio 2012 Release */

void __fastcall ~<>(int param_1)

{
  GFxSprite__unknown_014543f0((int *)(param_1 + 4));
  return;
}




/* ========== GImage.c ========== */

/*
 * SGW.exe - GImage (5 functions)
 */


/* public: __thiscall GImage::GImage(class GImageBase const &) */

GImage * __thiscall GImage::GImage(GImage *this,GImageBase *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
                    /* 0x16a2f0  5  ??0GImage@@QAE@ABVGImageBase@@@Z */
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0168e48b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  GRefCountBaseImpl::GRefCountBaseImpl((GRefCountBaseImpl *)this);
  *(undefined ***)this = GRefCountBase<class_GImage>::vftable;
  local_4 = 1;
  FUN_013c74c0(this + 0x10,(undefined4 *)param_1);
  *(undefined ***)this = vftable;
  ExceptionList = local_c;
  return this;
}




/* public: __thiscall GImage::GImage(class GImage const &) */

GImage * __thiscall GImage::GImage(GImage *this,GImage *param_1)

{
  GImage *pGVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
                    /* 0x16a3b0  4  ??0GImage@@QAE@ABV0@@Z */
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0168e48b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  GRefCountBaseImpl::GRefCountBaseImpl((GRefCountBaseImpl *)this);
  *(undefined ***)this = GRefCountBase<class_GImage>::vftable;
  local_4 = 1;
  if (param_1 == (GImage *)0x0) {
    pGVar1 = (GImage *)0x0;
  }
  else {
    pGVar1 = param_1 + 0x10;
  }
  FUN_013c74c0(this + 0x10,(undefined4 *)pGVar1);
  *(undefined ***)this = vftable;
  ExceptionList = local_c;
  return this;
}




/* public: __thiscall GImage::GImage(void) */

GImage * __thiscall GImage::GImage(GImage *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfc7aa0  7  ??0GImage@@QAE@XZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177dc08;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_005692a0((GRefCountBaseImpl *)this);
  local_8 = 0;
  FUN_013c7b50((int)(this + 0x10));
  *(undefined ***)this = vftable;
  *(undefined4 *)(this + 0x10) = 0;
  *(undefined4 *)(this + 0x1c) = 0;
  *(undefined4 *)(this + 0x18) = 0;
  *(undefined4 *)(this + 0x14) = 0;
  *(undefined4 *)(this + 0x20) = 0;
  *(undefined4 *)(this + 0x24) = 0;
  *(undefined4 *)(this + 0x28) = 1;
  ExceptionList = local_10;
  return this;
}




/* public: __thiscall GImage::GImage(enum GImageBase::ImageFormat,unsigned long,unsigned long) */

GImage * __thiscall GImage::GImage(GImage *this,ImageFormat param_1,ulong param_2,ulong param_3)

{
  uint uVar1;
  int iVar2;
  undefined4 uVar3;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfc7b70  6  ??0GImage@@QAE@W4ImageFormat@GImageBase@@KK@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177dc43;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_005692a0((GRefCountBaseImpl *)this);
  local_8 = 0;
  FUN_013c7b50((int)(this + 0x10));
  local_8 = CONCAT31(local_8._1_3_,1);
  *(undefined ***)this = vftable;
  uVar1 = FUN_013c7800(param_1,param_2);
  *(uint *)(this + 0x1c) = uVar1;
  iVar2 = FUN_013c78d0(param_1,param_2,param_3);
  *(int *)(this + 0x24) = iVar2;
  uVar3 = FUN_00455b00(*(undefined4 *)(this + 0x24));
  *(undefined4 *)(this + 0x20) = uVar3;
  if (*(int *)(this + 0x20) == 0) {
    FUN_013c7c70((undefined4 *)(this + 0x10));
  }
  else {
    *(ImageFormat *)(this + 0x10) = param_1;
    *(ulong *)(this + 0x14) = param_2;
    *(ulong *)(this + 0x18) = param_3;
    memset(*(void **)(this + 0x20),0,*(size_t *)(this + 0x24));
  }
  *(undefined4 *)(this + 0x28) = 1;
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GImage::~GImage(void) */

void __thiscall GImage::~GImage(GImage *this)

{
  GImage *local_18;
  void *local_10;
  undefined1 *puStack_c;
  uint local_8;
  
                    /* 0xfc7cd0  23  ??1GImage@@UAE@XZ */
  puStack_c = &LAB_0177dc8f;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  local_8 = 1;
  if (*(int *)(this + 0x20) != 0) {
    FUN_00455b50(*(undefined4 *)(this + 0x20));
  }
  local_8 = local_8 & 0xffffff00;
  if (this == (GImage *)0x0) {
    local_18 = (GImage *)0x0;
  }
  else {
    local_18 = this + 0x10;
  }
  FUN_0056a290((int)local_18);
  local_8 = 0xffffffff;
  FUN_00567f70((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* === Additional global functions for GImage (1 functions) === */

/* Also in vtable: GImage (slot 0) */

GImage * __thiscall GImage__vfunc_0(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    GImage::~GImage(this);
    if ((param_1 & 1) != 0) {
      FUN_00455b50(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x38,*(int *)((int)this + -4),GImage::~GImage);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((GImage *)((int)this + -4));
  }
  return (GImage *)((int)this + -4);
}




/* ========== GImageInfo.c ========== */

/*
 * SGW.exe - GImageInfo (1 functions)
 */

GRefCountBaseImpl * __thiscall GImageInfo__vfunc_0(void *this,uint param_1)

{
  FUN_013cf9c0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GImageInfoBase.c ========== */

/*
 * SGW.exe - GImageInfoBase (1 functions)
 */

/* Also in vtable: GImageInfoBase (slot 0) */

GRefCountBaseImpl * __thiscall GImageInfoBase__vfunc_0(void *this,byte param_1)

{
  FUN_00aefe20(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}




/* ========== GImageInfoBaseImpl.c ========== */

/*
 * SGW.exe - GImageInfoBaseImpl (1 functions)
 */

/* Also in vtable: GImageInfoBaseImpl (slot 0) */

GRefCountBaseImpl * __thiscall GImageInfoBaseImpl__vfunc_0(void *this,byte param_1)

{
  GImageInfoBaseImpl__unknown_013cf5e0(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}




/* ========== GJPEGInput.c ========== */

/*
 * SGW.exe - GJPEGInput (1 functions)
 */

undefined4 * __thiscall GJPEGInput__vfunc_0(void *this,uint param_1)

{
  FUN_013d4d40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GJPEGInputImpl.c ========== */

/*
 * SGW.exe - GJPEGInputImpl (1 functions)
 */

undefined4 * __thiscall GJPEGInputImpl__vfunc_0(void *this,uint param_1)

{
  FUN_013d4aa0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GMatrix2D.c ========== */

/*
 * SGW.exe - GMatrix2D (21 functions)
 */


/* public: void __thiscall GMatrix2D::Swap(class GMatrix2D *) */

void __thiscall GMatrix2D::Swap(GMatrix2D *this,GMatrix2D *param_1)

{
  undefined4 uVar1;
  
                    /* 0x169550  149  ?Swap@GMatrix2D@@QAEXPAV1@@Z */
  uVar1 = *(undefined4 *)this;
  *(undefined4 *)this = *(undefined4 *)param_1;
  *(undefined4 *)param_1 = uVar1;
  uVar1 = *(undefined4 *)(this + 4);
  *(undefined4 *)(this + 4) = *(undefined4 *)(param_1 + 4);
  *(undefined4 *)(param_1 + 4) = uVar1;
  uVar1 = *(undefined4 *)(this + 8);
  *(undefined4 *)(this + 8) = *(undefined4 *)(param_1 + 8);
  *(undefined4 *)(param_1 + 8) = uVar1;
  uVar1 = *(undefined4 *)(this + 0xc);
  *(undefined4 *)(this + 0xc) = *(undefined4 *)(param_1 + 0xc);
  *(undefined4 *)(param_1 + 0xc) = uVar1;
  uVar1 = *(undefined4 *)(this + 0x10);
  *(undefined4 *)(this + 0x10) = *(undefined4 *)(param_1 + 0x10);
  *(undefined4 *)(param_1 + 0x10) = uVar1;
  uVar1 = *(undefined4 *)(this + 0x14);
  *(undefined4 *)(this + 0x14) = *(undefined4 *)(param_1 + 0x14);
  *(undefined4 *)(param_1 + 0x14) = uVar1;
  return;
}




GMatrix2D * __thiscall GMatrix2D::Prepend(GMatrix2D *this,GMatrix2D *param_1)

{
  float fStack_1c;
  float fStack_18;
  float fStack_14;
  float fStack_10;
  float fStack_c;
  float fStack_8;
  
  GFxSprite__unknown_00af2150(&fStack_1c,(undefined4 *)this);
  *(float *)this = fStack_18 * *(float *)(param_1 + 0xc) + fStack_1c * *(float *)param_1;
  *(float *)(this + 0xc) = fStack_c * *(float *)(param_1 + 0xc) + fStack_10 * *(float *)param_1;
  *(float *)(this + 4) =
       fStack_18 * *(float *)(param_1 + 0x10) + fStack_1c * *(float *)(param_1 + 4);
  *(float *)(this + 0x10) =
       fStack_c * *(float *)(param_1 + 0x10) + fStack_10 * *(float *)(param_1 + 4);
  *(float *)(this + 8) =
       fStack_18 * *(float *)(param_1 + 0x14) + fStack_1c * *(float *)(param_1 + 8) + fStack_14;
  *(float *)(this + 0x14) =
       fStack_c * *(float *)(param_1 + 0x14) + fStack_10 * *(float *)(param_1 + 8) + fStack_8;
  return this;
}




/* WARNING: Removing unreachable block (ram,0x013d2ee0) */
/* WARNING: Removing unreachable block (ram,0x013d2ef3) */
/* WARNING: Removing unreachable block (ram,0x013d2f0f) */
/* WARNING: Removing unreachable block (ram,0x013d2f06) */
/* WARNING: Removing unreachable block (ram,0x013d2f16) */
/* WARNING: Removing unreachable block (ram,0x013d2dc6) */
/* WARNING: Removing unreachable block (ram,0x013d2dd9) */
/* WARNING: Removing unreachable block (ram,0x013d2df5) */
/* WARNING: Removing unreachable block (ram,0x013d2dec) */
/* WARNING: Removing unreachable block (ram,0x013d2dfc) */
/* WARNING: Removing unreachable block (ram,0x013d2cae) */
/* WARNING: Removing unreachable block (ram,0x013d2cc0) */
/* WARNING: Removing unreachable block (ram,0x013d2cdb) */
/* WARNING: Removing unreachable block (ram,0x013d2cd2) */
/* WARNING: Removing unreachable block (ram,0x013d2ce2) */
/* WARNING: Removing unreachable block (ram,0x013d2d39) */
/* WARNING: Removing unreachable block (ram,0x013d2d4c) */
/* WARNING: Removing unreachable block (ram,0x013d2d68) */
/* WARNING: Removing unreachable block (ram,0x013d2d5f) */
/* WARNING: Removing unreachable block (ram,0x013d2d6f) */
/* WARNING: Removing unreachable block (ram,0x013d2e53) */
/* WARNING: Removing unreachable block (ram,0x013d2e66) */
/* WARNING: Removing unreachable block (ram,0x013d2e82) */
/* WARNING: Removing unreachable block (ram,0x013d2e79) */
/* WARNING: Removing unreachable block (ram,0x013d2e89) */
/* WARNING: Removing unreachable block (ram,0x013d2f6d) */
/* WARNING: Removing unreachable block (ram,0x013d2f80) */
/* WARNING: Removing unreachable block (ram,0x013d2f9c) */
/* WARNING: Removing unreachable block (ram,0x013d2f93) */
/* WARNING: Removing unreachable block (ram,0x013d2fa3) */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: bool __thiscall GMatrix2D::IsValid(void)const  */

bool __thiscall GMatrix2D::IsValid(GMatrix2D *this)

{
  bool bVar1;
  
                    /* 0xfd2c60  104  ?IsValid@GMatrix2D@@QBE_NXZ */
  if ((*(float *)this < (float)_DAT_01aedb40) ||
     (*(float *)this < (float)_DAT_01ac8ee0 == (*(float *)this == (float)_DAT_01ac8ee0))) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  if (bVar1) {
    if ((*(float *)(this + 4) < (float)_DAT_01aedb40) ||
       (*(float *)(this + 4) < (float)_DAT_01ac8ee0 ==
        (*(float *)(this + 4) == (float)_DAT_01ac8ee0))) {
      bVar1 = false;
    }
    else {
      bVar1 = true;
    }
    if (bVar1) {
      if ((*(float *)(this + 8) < (float)_DAT_01aedb40) ||
         (*(float *)(this + 8) < (float)_DAT_01ac8ee0 ==
          (*(float *)(this + 8) == (float)_DAT_01ac8ee0))) {
        bVar1 = false;
      }
      else {
        bVar1 = true;
      }
      if (bVar1) {
        if ((*(float *)(this + 0xc) < (float)_DAT_01aedb40) ||
           (*(float *)(this + 0xc) < (float)_DAT_01ac8ee0 ==
            (*(float *)(this + 0xc) == (float)_DAT_01ac8ee0))) {
          bVar1 = false;
        }
        else {
          bVar1 = true;
        }
        if (bVar1) {
          if ((*(float *)(this + 0x10) < (float)_DAT_01aedb40) ||
             (*(float *)(this + 0x10) < (float)_DAT_01ac8ee0 ==
              (*(float *)(this + 0x10) == (float)_DAT_01ac8ee0))) {
            bVar1 = false;
          }
          else {
            bVar1 = true;
          }
          if (bVar1) {
            if ((*(float *)(this + 0x14) < (float)_DAT_01aedb40) ||
               (*(float *)(this + 0x14) < (float)_DAT_01ac8ee0 ==
                (*(float *)(this + 0x14) == (float)_DAT_01ac8ee0))) {
              bVar1 = false;
            }
            else {
              bVar1 = true;
            }
            if (bVar1) {
              return true;
            }
          }
        }
      }
    }
  }
  return false;
}




/* public: void __thiscall GMatrix2D::SetIdentity(void) */

void __thiscall GMatrix2D::SetIdentity(GMatrix2D *this)

{
                    /* 0xfd2fd0  140  ?SetIdentity@GMatrix2D@@QAEXXZ */
  *(undefined4 *)this = 0x3f800000;
  *(undefined4 *)(this + 4) = 0;
  *(undefined4 *)(this + 8) = 0;
  *(undefined4 *)(this + 0xc) = 0;
  *(undefined4 *)(this + 0x10) = 0x3f800000;
  *(undefined4 *)(this + 0x14) = 0;
  return;
}




/* public: class GMatrix2D & __thiscall GMatrix2D::Prepend(class GMatrix2D const &) */

GMatrix2D * __thiscall GMatrix2D::Prepend(GMatrix2D *this,GMatrix2D *param_1)

{
  float local_1c;
  float local_18;
  float local_14;
  float local_10;
  float local_c;
  float local_8;
  
                    /* 0xfd3010  121  ?Prepend@GMatrix2D@@QAEAAV1@ABV1@@Z */
  GFxSprite__unknown_00af2150(&local_1c,(undefined4 *)this);
  *(float *)this = local_18 * *(float *)(param_1 + 0xc) + local_1c * *(float *)param_1;
  *(float *)(this + 0xc) = local_c * *(float *)(param_1 + 0xc) + local_10 * *(float *)param_1;
  *(float *)(this + 4) = local_18 * *(float *)(param_1 + 0x10) + local_1c * *(float *)(param_1 + 4);
  *(float *)(this + 0x10) =
       local_c * *(float *)(param_1 + 0x10) + local_10 * *(float *)(param_1 + 4);
  *(float *)(this + 8) =
       local_18 * *(float *)(param_1 + 0x14) + local_1c * *(float *)(param_1 + 8) + local_14;
  *(float *)(this + 0x14) =
       local_c * *(float *)(param_1 + 0x14) + local_10 * *(float *)(param_1 + 8) + local_8;
  return this;
}




/* public: class GMatrix2D & __thiscall GMatrix2D::Append(class GMatrix2D const &) */

GMatrix2D * __thiscall GMatrix2D::Append(GMatrix2D *this,GMatrix2D *param_1)

{
  float local_1c;
  float local_18;
  float local_14;
  float local_10;
  float local_c;
  float local_8;
  
                    /* 0xfd30d0  43  ?Append@GMatrix2D@@QAEAAV1@ABV1@@Z */
  GFxSprite__unknown_00af2150(&local_1c,(undefined4 *)this);
  *(float *)this = *(float *)(param_1 + 4) * local_10 + *(float *)param_1 * local_1c;
  *(float *)(this + 0xc) =
       *(float *)(param_1 + 0x10) * local_10 + *(float *)(param_1 + 0xc) * local_1c;
  *(float *)(this + 4) = *(float *)(param_1 + 4) * local_c + *(float *)param_1 * local_18;
  *(float *)(this + 0x10) =
       *(float *)(param_1 + 0x10) * local_c + *(float *)(param_1 + 0xc) * local_18;
  *(float *)(this + 8) =
       *(float *)(param_1 + 4) * local_8 + *(float *)param_1 * local_14 + *(float *)(param_1 + 8);
  *(float *)(this + 0x14) =
       *(float *)(param_1 + 0x10) * local_8 + *(float *)(param_1 + 0xc) * local_14 +
       *(float *)(param_1 + 0x14);
  return this;
}




/* public: void __thiscall GMatrix2D::SetLerp(class GMatrix2D const &,class GMatrix2D const &,float)
    */

void __thiscall
GMatrix2D::SetLerp(GMatrix2D *this,GMatrix2D *param_1,GMatrix2D *param_2,float param_3)

{
  float10 fVar1;
  
                    /* 0xfd31a0  142  ?SetLerp@GMatrix2D@@QAEXABV1@0M@Z */
  fVar1 = GSemaphoreWaitableIncrement__unknown_00af2120(*(float *)param_1,*(float *)param_2,param_3)
  ;
  *(float *)this = (float)fVar1;
  fVar1 = GSemaphoreWaitableIncrement__unknown_00af2120
                    (*(float *)(param_1 + 0xc),*(float *)(param_2 + 0xc),param_3);
  *(float *)(this + 0xc) = (float)fVar1;
  fVar1 = GSemaphoreWaitableIncrement__unknown_00af2120
                    (*(float *)(param_1 + 4),*(float *)(param_2 + 4),param_3);
  *(float *)(this + 4) = (float)fVar1;
  fVar1 = GSemaphoreWaitableIncrement__unknown_00af2120
                    (*(float *)(param_1 + 0x10),*(float *)(param_2 + 0x10),param_3);
  *(float *)(this + 0x10) = (float)fVar1;
  fVar1 = GSemaphoreWaitableIncrement__unknown_00af2120
                    (*(float *)(param_1 + 8),*(float *)(param_2 + 8),param_3);
  *(float *)(this + 8) = (float)fVar1;
  fVar1 = GSemaphoreWaitableIncrement__unknown_00af2120
                    (*(float *)(param_1 + 0x14),*(float *)(param_2 + 0x14),param_3);
  *(float *)(this + 0x14) = (float)fVar1;
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: void __thiscall GMatrix2D::Format(char *)const  */

void __thiscall GMatrix2D::Format(GMatrix2D *this,char *param_1)

{
                    /* 0xfd32a0  71  ?Format@GMatrix2D@@QBEXPAD@Z */
  FUN_013cab40(param_1,0x200,"| %4.4f %4.4f %4.4f |\n| %4.4f %4.4f %4.4f |\n");
  return;
}




/* public: void __thiscall GMatrix2D::Transform(class GPoint<float> *,class GPoint<float> const
   &)const  */

void __thiscall GMatrix2D::Transform(GMatrix2D *this,GPoint<float> *param_1,GPoint<float> *param_2)

{
                    /* 0xfd3320  152  ?Transform@GMatrix2D@@QBEXPAV?$GPoint@M@@ABV2@@Z */
  *(float *)param_1 =
       *(float *)(this + 4) * *(float *)(param_2 + 4) + *(float *)this * *(float *)param_2 +
       *(float *)(this + 8);
  *(float *)(param_1 + 4) =
       *(float *)(this + 0x10) * *(float *)(param_2 + 4) +
       *(float *)(this + 0xc) * *(float *)param_2 + *(float *)(this + 0x14);
  return;
}




/* public: void __thiscall GMatrix2D::TransformVector(class GPoint<float> *,class GPoint<float>
   const &)const  */

void __thiscall
GMatrix2D::TransformVector(GMatrix2D *this,GPoint<float> *param_1,GPoint<float> *param_2)

{
                    /* 0xfd3380  154  ?TransformVector@GMatrix2D@@QBEXPAV?$GPoint@M@@ABV2@@Z */
  *(float *)param_1 =
       *(float *)(this + 4) * *(float *)(param_2 + 4) + *(float *)this * *(float *)param_2;
  *(float *)(param_1 + 4) =
       *(float *)(this + 0x10) * *(float *)(param_2 + 4) +
       *(float *)(this + 0xc) * *(float *)param_2;
  return;
}




/* public: void __thiscall GMatrix2D::TransformByInverse(class GPoint<float> *,class GPoint<float>
   const &)const  */

void __thiscall
GMatrix2D::TransformByInverse(GMatrix2D *this,GPoint<float> *param_1,GPoint<float> *param_2)

{
  GMatrix2D *pGVar1;
  undefined1 local_1c [24];
  
                    /* 0xfd33d0  153  ?TransformByInverse@GMatrix2D@@QBEXPAV?$GPoint@M@@ABV2@@Z */
  pGVar1 = (GMatrix2D *)GFxSprite__unknown_00af2150(local_1c,(undefined4 *)this);
  pGVar1 = FUN_013d3410(pGVar1);
  Transform(pGVar1,param_1,param_2);
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: void __thiscall GMatrix2D::SetInverse(class GMatrix2D const &) */

void __thiscall GMatrix2D::SetInverse(GMatrix2D *this,GMatrix2D *param_1)

{
  float fVar1;
  
                    /* 0xfd3440  141  ?SetInverse@GMatrix2D@@QAEXABV1@@Z */
  fVar1 = *(float *)param_1 * *(float *)(param_1 + 0x10) -
          *(float *)(param_1 + 4) * *(float *)(param_1 + 0xc);
  if (fVar1 == (float)_DAT_017f9268) {
    SetIdentity(this);
    *(float *)(this + 8) = -*(float *)(param_1 + 8);
    *(float *)(this + 0x14) = -*(float *)(param_1 + 0x14);
  }
  else {
    fVar1 = 1.0 / fVar1;
    *(float *)this = *(float *)(param_1 + 0x10) * fVar1;
    *(float *)(this + 0x10) = *(float *)param_1 * fVar1;
    *(float *)(this + 4) = -*(float *)(param_1 + 4) * fVar1;
    *(float *)(this + 0xc) = -*(float *)(param_1 + 0xc) * fVar1;
    *(float *)(this + 8) =
         -(*(float *)(this + 4) * *(float *)(param_1 + 0x14) +
          *(float *)this * *(float *)(param_1 + 8));
    *(float *)(this + 0x14) =
         -(*(float *)(this + 0x10) * *(float *)(param_1 + 0x14) +
          *(float *)(this + 0xc) * *(float *)(param_1 + 8));
  }
  return;
}




/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* public: bool __thiscall GMatrix2D::DoesFlip(void)const  */

bool __thiscall GMatrix2D::DoesFlip(GMatrix2D *this)

{
                    /* 0xfd3530  63  ?DoesFlip@GMatrix2D@@QBE_NXZ */
  return *(float *)this * *(float *)(this + 0x10) - *(float *)(this + 4) * *(float *)(this + 0xc) <
         (float)_DAT_017f9268;
}




/* public: float __thiscall GMatrix2D::GetDeterminant(void)const  */

float __thiscall GMatrix2D::GetDeterminant(GMatrix2D *this)

{
                    /* 0xfd3580  78  ?GetDeterminant@GMatrix2D@@QBEMXZ */
  return *(float *)this * *(float *)(this + 0x10) - *(float *)(this + 0xc) * *(float *)(this + 4);
}




/* public: float __thiscall GMatrix2D::GetMaxScale(void)const  */

float __thiscall GMatrix2D::GetMaxScale(GMatrix2D *this)

{
  float10 fVar1;
  
                    /* 0xfd35b0  90  ?GetMaxScale@GMatrix2D@@QBEMXZ */
  GFxTextDocView__unknown_00aef3e0
            (*(float *)(this + 0xc) * *(float *)(this + 0xc) + *(float *)this * *(float *)this,
             *(float *)(this + 0x10) * *(float *)(this + 0x10) +
             *(float *)(this + 4) * *(float *)(this + 4));
  fVar1 = GFxSprite__unknown_004024b0();
  return (float)fVar1;
}




/* public: double __thiscall GMatrix2D::GetXScale(void)const  */

double __thiscall GMatrix2D::GetXScale(GMatrix2D *this)

{
  double dVar1;
  
                    /* 0xfd3620  94  ?GetXScale@GMatrix2D@@QBENXZ */
  dVar1 = sqrt((double)(*(float *)(this + 0xc) * *(float *)(this + 0xc) +
                       *(float *)this * *(float *)this));
  return dVar1;
}




/* public: double __thiscall GMatrix2D::GetYScale(void)const  */

double __thiscall GMatrix2D::GetYScale(GMatrix2D *this)

{
  double dVar1;
  
                    /* 0xfd3660  96  ?GetYScale@GMatrix2D@@QBENXZ */
  dVar1 = sqrt((double)(*(float *)(this + 4) * *(float *)(this + 4) +
                       *(float *)(this + 0x10) * *(float *)(this + 0x10)));
  return dVar1;
}




/* public: double __thiscall GMatrix2D::GetRotation(void)const  */

double __thiscall GMatrix2D::GetRotation(GMatrix2D *this)

{
  double dVar1;
  
                    /* 0xfd36a0  91  ?GetRotation@GMatrix2D@@QBENXZ */
  dVar1 = atan2((double)*(float *)(this + 0xc),(double)*(float *)this);
  return dVar1;
}




/* public: void __thiscall GMatrix2D::EncloseTransform(class GRect<float> *,class GRect<float> const
   &)const  */

void __thiscall
GMatrix2D::EncloseTransform(GMatrix2D *this,GRect<float> *param_1,GRect<float> *param_2)

{
  GPoint<float> *pGVar1;
  undefined1 local_44 [8];
  undefined1 local_3c [8];
  undefined1 local_34 [8];
  undefined1 local_2c [8];
  IUnknown local_24 [2];
  IUnknown local_1c [2];
  IUnknown local_14 [2];
  IUnknown local_c [2];
  
                    /* 0xfd36d0  64  ?EncloseTransform@GMatrix2D@@QBEXPAV?$GRect@M@@ABV2@@Z */
  IUnknown::IUnknown(local_c);
  IUnknown::IUnknown(local_24);
  IUnknown::IUnknown(local_14);
  IUnknown::IUnknown(local_1c);
  pGVar1 = FUN_013d39e0(param_2,local_2c);
  Transform(this,(GPoint<float> *)local_c,pGVar1);
  pGVar1 = FUN_013d3a10(param_2,local_34);
  Transform(this,(GPoint<float> *)local_24,pGVar1);
  pGVar1 = FUN_013d3a70(param_2,local_3c);
  Transform(this,(GPoint<float> *)local_14,pGVar1);
  pGVar1 = FUN_013d3a40(param_2,local_44);
  Transform(this,(GPoint<float> *)local_1c,pGVar1);
  FUN_013d39a0(param_1,local_c,local_c);
  FUN_013d3aa0(param_1,(float *)local_24);
  FUN_013d3aa0(param_1,(float *)local_14);
  FUN_013d3aa0(param_1,(float *)local_1c);
  return;
}




/* public: float __thiscall GMatrix2D::GetX(void)const  */

float __thiscall GMatrix2D::GetX(GMatrix2D *this)

{
                    /* 0x1044d70  93  ?GetX@GMatrix2D@@QBEMXZ */
  return *(float *)(this + 8);
}




/* public: float __thiscall GMatrix2D::GetY(void)const  */

float __thiscall GMatrix2D::GetY(GMatrix2D *this)

{
                    /* 0x10885f0  95  ?GetY@GMatrix2D@@QBEMXZ */
  return *(float *)(this + 0x14);
}





/* ========== GMutex.c ========== */

/*
 * SGW.exe - GMutex (12 functions)
 */


/* public: void __thiscall GMutex::`default constructor closure'(void) */

void __thiscall GMutex::_default_constructor_closure_(GMutex *this)

{
                    /* 0x1cec0  35  ??_FGMutex@@QAEXXZ */
  GMutex(this,true,false);
  return;
}




/* public: __thiscall GMutex::GMutex(bool,bool) */

GMutex * __thiscall GMutex::GMutex(GMutex *this,bool param_1,bool param_2)

{
  void *this_00;
  undefined4 *local_20;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x53d10  8  ??0GMutex@@QAE@_N0@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167adee;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004514d0(this,param_2);
  local_8 = 0;
  FUN_00451db0((undefined4 *)(this + 0x14));
  local_8._0_1_ = 1;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  this_00 = (void *)GFxSprite__unknown_0146a300(0x14);
  local_8 = CONCAT31(local_8._1_3_,2);
  if (this_00 == (void *)0x0) {
    local_20 = (undefined4 *)0x0;
  }
  else {
    local_20 = FUN_00453a00(this_00,this,param_1);
  }
  *(undefined4 **)(this + 0x18) = local_20;
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GMutex::~GMutex(void) */

void __thiscall GMutex::~GMutex(GMutex *this)

{
  GAcquireInterface *local_24;
  void *local_10;
  undefined1 *puStack_c;
  uint local_8;
  
                    /* 0x53e40  24  ??1GMutex@@UAE@XZ */
  puStack_c = &LAB_0167ae3f;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  local_8 = 1;
  if (*(void **)(this + 0x18) != (void *)0x0) {
    FUN_00453ef0(*(void **)(this + 0x18),1);
  }
  local_8 = local_8 & 0xffffff00;
  if (this == (GMutex *)0x0) {
    local_24 = (GAcquireInterface *)0x0;
  }
  else {
    local_24 = (GAcquireInterface *)(this + 0x14);
  }
  GAcquireInterface::~GAcquireInterface(local_24);
  local_8 = 0xffffffff;
  FUN_00451650((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* public: void __thiscall GMutex::Lock(void) */

void __thiscall GMutex::Lock(GMutex *this)

{
                    /* 0x53f20  114  ?Lock@GMutex@@QAEXXZ */
  FUN_00453b30(*(undefined4 **)(this + 0x18));
  return;
}




/* public: bool __thiscall GMutex::TryLock(void) */

bool __thiscall GMutex::TryLock(GMutex *this)

{
  uint uVar1;
  
                    /* 0x53f40  167  ?TryLock@GMutex@@QAE_NXZ */
  uVar1 = FUN_00453b60(*(undefined4 **)(this + 0x18));
  return SUB41(uVar1,0);
}




/* public: void __thiscall GMutex::Unlock(void) */

void __thiscall GMutex::Unlock(GMutex *this)

{
                    /* 0x53f60  168  ?Unlock@GMutex@@QAEXXZ */
  FUN_00453ba0(*(void **)(this + 0x18),this);
  return;
}




/* public: bool __thiscall GMutex::IsLockedByAnotherThread(void) */

bool __thiscall GMutex::IsLockedByAnotherThread(GMutex *this)

{
  undefined4 uVar1;
  
                    /* 0x53f80  98  ?IsLockedByAnotherThread@GMutex@@QAE_NXZ */
  uVar1 = FUN_00453c60(*(void **)(this + 0x18),this);
  return SUB41(uVar1,0);
}




/* public: virtual bool __thiscall GMutex::IsSignaled(void)const  */

bool __thiscall GMutex::IsSignaled(GMutex *this)

{
  bool bVar1;
  
                    /* 0x53fa0  100  ?IsSignaled@GMutex@@UBE_NXZ */
  bVar1 = FUN_00453ca0(*(int *)(this + 0x18));
  return bVar1;
}




/* public: virtual class GAcquireInterface * __thiscall GMutex::GetAcquireInterface(void) */

GAcquireInterface * __thiscall GMutex::GetAcquireInterface(GMutex *this)

{
  GAcquireInterface *pGVar1;
  
                    /* 0x53fc0  73  ?GetAcquireInterface@GMutex@@UAEPAVGAcquireInterface@@XZ */
  pGVar1 = (GAcquireInterface *)FUN_00453cc0(*(void **)(this + 0x18),this);
  return pGVar1;
}




/* public: virtual bool __thiscall GMutex::CanAcquire(void) */

bool __thiscall GMutex::CanAcquire(GMutex *this)

{
  bool bVar1;
  
                    /* 0x53fe0  51  ?CanAcquire@GMutex@@UAE_NXZ */
  bVar1 = IsLockedByAnotherThread(this + -0x14);
  return (bool)('\x01' - bVar1);
}




/* public: virtual bool __thiscall GMutex::TryAcquire(void) */

bool __thiscall GMutex::TryAcquire(GMutex *this)

{
  bool bVar1;
  
                    /* 0x54000  157  ?TryAcquire@GMutex@@UAE_NXZ */
  bVar1 = TryLock(this + -0x14);
  return bVar1;
}




/* public: virtual bool __thiscall GMutex::TryAcquireCancel(void) */

bool __thiscall GMutex::TryAcquireCancel(GMutex *this)

{
                    /* 0x54020  161  ?TryAcquireCancel@GMutex@@UAE_NXZ */
  Unlock(this + -0x14);
  return true;
}




/* === Additional global functions for GMutex (1 functions) === */

/* Also in vtable: GMutex (slot 0) */

GMutex * __thiscall GMutex__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GMutex::~GMutex(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_(this,0x1c,*(int *)((int)this + -4),GMutex::~GMutex);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== GMutex_AreadyLockedAcquireInterface.c ========== */

/*
 * SGW.exe - GMutex_AreadyLockedAcquireInterface (1 functions)
 */

/* Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 0) */

GAcquireInterface * __thiscall GMutex_AreadyLockedAcquireInterface__vfunc_0(void *this,uint param_1)

{
  FUN_00453ae0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GRefCountBaseImpl.c ========== */

/*
 * SGW.exe - GRefCountBaseImpl (4 functions)
 */


/* public: __thiscall GRefCountBaseImpl::GRefCountBaseImpl(void) */

GRefCountBaseImpl * __thiscall GRefCountBaseImpl::GRefCountBaseImpl(GRefCountBaseImpl *this)

{
                    /* 0x55740  10  ??0GRefCountBaseImpl@@QAE@XZ */
  *(undefined ***)this = vftable;
  *(undefined4 *)(this + 0xc) = 0;
  if ((*(int *)(this + 4) == 0x56471e89) && (*(int *)(this + 8) == -0x601edcb6)) {
    *(undefined4 *)(this + 4) = 1;
    *(undefined ***)(this + 8) = &PTR_FUN_01dadfc8;
  }
  else {
    *(undefined4 *)(this + 4) = 0;
    *(undefined ***)(this + 8) = &PTR_OnExit_01dadfc0;
  }
  return this;
}




/* public: __thiscall GRefCountBaseImpl::GRefCountBaseImpl(class GRefCountImpl *,class GRefCountImpl
   *) */

GRefCountBaseImpl * __thiscall
GRefCountBaseImpl::GRefCountBaseImpl
          (GRefCountBaseImpl *this,GRefCountImpl *param_1,GRefCountImpl *param_2)

{
  GRefCountImpl *local_c;
  
                    /* 0x55820  9  ??0GRefCountBaseImpl@@QAE@PAVGRefCountImpl@@0@Z */
  *(undefined ***)this = vftable;
  *(undefined4 *)(this + 0xc) = 0;
  if ((*(int *)(this + 4) == 0x56471e89) && (*(int *)(this + 8) == -0x601edcb6)) {
    *(undefined4 *)(this + 4) = 1;
    *(GRefCountImpl **)(this + 8) = param_1;
  }
  else {
    *(undefined4 *)(this + 4) = 0;
    if (param_2 == (GRefCountImpl *)0x0) {
      local_c = (GRefCountImpl *)&PTR_OnExit_01dadfc0;
    }
    else {
      local_c = param_2;
    }
    *(GRefCountImpl **)(this + 8) = local_c;
  }
  return this;
}




/* public: virtual __thiscall GRefCountBaseImpl::~GRefCountBaseImpl(void) */

void __thiscall GRefCountBaseImpl::~GRefCountBaseImpl(GRefCountBaseImpl *this)

{
                    /* 0x558a0  25  ??1GRefCountBaseImpl@@UAE@XZ */
  *(undefined ***)this = vftable;
  if (*(int *)(this + 0xc) != 0) {
    FUN_004558e0(*(int *)(this + 0xc));
    FUN_00455900(*(int **)(this + 0xc));
  }
  return;
}




/* public: bool __thiscall GRefCountBaseImpl::SetRefCountMode(unsigned int) */

bool __thiscall GRefCountBaseImpl::SetRefCountMode(GRefCountBaseImpl *this,uint param_1)

{
  bool bVar1;
  
                    /* 0x55940  143  ?SetRefCountMode@GRefCountBaseImpl@@QAE_NI@Z */
  if (*(undefined ***)(this + 8) == &PTR_OnExit_01dadfc0) {
    bVar1 = false;
  }
  else if (param_1 == 0) {
    *(undefined ***)(this + 8) = &PTR_FUN_01dadfc8;
    bVar1 = true;
  }
  else if (param_1 == 1) {
    *(undefined ***)(this + 8) = &PTR_FUN_01dadfd0;
    bVar1 = true;
  }
  else {
    bVar1 = false;
  }
  return bVar1;
}




/* === Additional global functions for GRefCountBaseImpl (1 functions) === */

/* Also in vtable: GRefCountBaseImpl (slot 0) */

GRefCountBaseImpl * __thiscall GRefCountBaseImpl__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GRefCountBaseImpl::~GRefCountBaseImpl(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_
              (this,0x10,*(int *)((int)this + -4),GRefCountBaseImpl::~GRefCountBaseImpl);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== GRenderer.c ========== */

/*
 * SGW.exe - GRenderer (1 functions)
 */

/* Also in vtable: GRenderer (slot 0) */

GRefCountBaseImpl * __thiscall GRenderer__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016daa70;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = GRenderer::vftable;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== GRenderer_EventHandler.c ========== */

/*
 * SGW.exe - GRenderer_EventHandler (1 functions)
 */

undefined4 * __thiscall GRenderer_EventHandler__vfunc_0(void *this,uint param_1)

{
  FUN_013fc270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GSemaphore.c ========== */

/*
 * SGW.exe - GSemaphore (14 functions)
 */


/* public: void __thiscall GSemaphore::`default constructor closure'(void) */

void __thiscall GSemaphore::_default_constructor_closure_(GSemaphore *this)

{
                    /* 0x1cee0  36  ??_FGSemaphore@@QAEXXZ */
  GSemaphore(this,1,false);
  return;
}




/* public: __thiscall GSemaphore::GSemaphore(int,bool) */

GSemaphore * __thiscall GSemaphore::GSemaphore(GSemaphore *this,int param_1,bool param_2)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x52b60  11  ??0GSemaphore@@QAE@H_N@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167ac3e;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004514d0(this,param_2);
  local_8 = 0;
  FUN_00451db0((undefined4 *)(this + 0x14));
  local_8._0_1_ = 1;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  GMutex::GMutex((GMutex *)(this + 0x20),true,false);
  local_8 = CONCAT31(local_8._1_3_,2);
  GWaitCondition::GWaitCondition((GWaitCondition *)(this + 0x3c));
  *(int *)(this + 0x18) = param_1;
  *(undefined4 *)(this + 0x1c) = 0;
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GSemaphore::~GSemaphore(void) */

void __thiscall GSemaphore::~GSemaphore(GSemaphore *this)

{
  GAcquireInterface *local_18;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
                    /* 0x52c80  26  ??1GSemaphore@@UAE@XZ */
  puStack_c = &LAB_0167ac9a;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  local_8 = 2;
  GWaitCondition::~GWaitCondition((GWaitCondition *)(this + 0x3c));
  local_8._0_1_ = 1;
  GMutex::~GMutex((GMutex *)(this + 0x20));
  local_8 = (uint)local_8._1_3_ << 8;
  if (this == (GSemaphore *)0x0) {
    local_18 = (GAcquireInterface *)0x0;
  }
  else {
    local_18 = (GAcquireInterface *)(this + 0x14);
  }
  GAcquireInterface::~GAcquireInterface(local_18);
  local_8 = 0xffffffff;
  FUN_00451650((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* public: bool __thiscall GSemaphore::ObtainSemaphore(int,unsigned int) */

bool __thiscall GSemaphore::ObtainSemaphore(GSemaphore *this,int param_1,uint param_2)

{
  bool bVar1;
  int iVar2;
  int iVar3;
  undefined4 local_18;
  uint local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x52d40  118  ?ObtainSemaphore@GSemaphore@@QAE_NHI@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167acc8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  if (*(int *)(this + 0x18) < param_1) {
    bVar1 = false;
  }
  else {
    FUN_004528a0(&local_18,this + 0x20);
    local_8 = 0;
    if (*(int *)(this + 0x18) < *(int *)(this + 0x1c) + param_1) {
      if (param_2 == 0) {
        local_8 = 0xffffffff;
        FUN_004528d0(&local_18);
        bVar1 = false;
      }
      else if (param_2 == 0xffffffff) {
        while (*(int *)(this + 0x18) < *(int *)(this + 0x1c) + param_1) {
          GWaitCondition::Wait((GWaitCondition *)(this + 0x3c),(GMutex *)(this + 0x20),0xffffffff);
        }
        *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_1;
        local_8 = 0xffffffff;
        FUN_004528d0(&local_18);
        bVar1 = true;
      }
      else {
        local_14 = param_2;
        iVar2 = FUN_004560c0();
        while (bVar1 = GWaitCondition::Wait
                                 ((GWaitCondition *)(this + 0x3c),(GMutex *)(this + 0x20),local_14),
              bVar1) {
          if (*(int *)(this + 0x1c) + param_1 <= *(int *)(this + 0x18)) {
            *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_1;
            local_8 = 0xffffffff;
            FUN_004528d0(&local_18);
            ExceptionList = local_10;
            return true;
          }
          iVar3 = FUN_004560c0();
          if (param_2 <= (uint)(iVar3 - iVar2)) break;
          local_14 = param_2 - (iVar3 - iVar2);
        }
        local_8 = 0xffffffff;
        FUN_004528d0(&local_18);
        bVar1 = false;
      }
    }
    else {
      *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_1;
      local_8 = 0xffffffff;
      FUN_004528d0(&local_18);
      bVar1 = true;
    }
  }
  ExceptionList = local_10;
  return bVar1;
}




/* public: bool __thiscall GSemaphore::ReleaseSemaphore(int) */

bool __thiscall GSemaphore::ReleaseSemaphore(GSemaphore *this,int param_1)

{
  cancellation_token_source local_14 [4];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x52f00  126  ?ReleaseSemaphore@GSemaphore@@QAE_NH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167acf8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  if (param_1 != 0) {
    GMutex::Lock((GMutex *)(this + 0x20));
    if (*(int *)(this + 0x1c) - param_1 < 0) {
      *(undefined4 *)(this + 0x1c) = 0;
    }
    else {
      *(int *)(this + 0x1c) = *(int *)(this + 0x1c) - param_1;
    }
    if (param_1 == 1) {
      GWaitCondition::Notify((GWaitCondition *)(this + 0x3c));
    }
    else {
      GWaitCondition::NotifyAll((GWaitCondition *)(this + 0x3c));
    }
    FUN_014babb0((undefined4 *)local_14);
    local_8 = 0;
    FUN_004529c0(this,local_14);
    GMutex::Unlock((GMutex *)(this + 0x20));
    FUN_00452990((undefined4 *)local_14);
    local_8 = 0xffffffff;
    FUN_004529f0(local_14);
  }
  ExceptionList = local_10;
  return true;
}




/* public: int __thiscall GSemaphore::operator++(int) */

int __thiscall GSemaphore::operator++(GSemaphore *this,int param_1)

{
  int iVar1;
  undefined4 local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x52fe0  30  ??EGSemaphore@@QAEHH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167ad28;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004528a0(&local_14,this + 0x20);
  local_8 = 0;
  while (*(int *)(this + 0x18) <= *(int *)(this + 0x1c)) {
    GWaitCondition::Wait((GWaitCondition *)(this + 0x3c),(GMutex *)(this + 0x20),0xffffffff);
  }
  *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + 1;
  iVar1 = *(int *)(this + 0x1c);
  local_8 = 0xffffffff;
  FUN_004528d0(&local_14);
  ExceptionList = local_10;
  return iVar1;
}




/* public: int __thiscall GSemaphore::operator--(int) */

int __thiscall GSemaphore::operator--(GSemaphore *this,int param_1)

{
  int iVar1;
  cancellation_token_source local_14 [4];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x53080  31  ??FGSemaphore@@QAEHH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167acf8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  GMutex::Lock((GMutex *)(this + 0x20));
  if (0 < *(int *)(this + 0x1c)) {
    *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + -1;
  }
  GWaitCondition::Notify((GWaitCondition *)(this + 0x3c));
  FUN_014babb0((undefined4 *)local_14);
  local_8 = 0;
  FUN_004529c0(this,local_14);
  GMutex::Unlock((GMutex *)(this + 0x20));
  FUN_00452990((undefined4 *)local_14);
  iVar1 = *(int *)(this + 0x1c);
  local_8 = 0xffffffff;
  FUN_004529f0(local_14);
  ExceptionList = local_10;
  return iVar1;
}




/* public: int __thiscall GSemaphore::operator+=(int) */

int __thiscall GSemaphore::operator+=(GSemaphore *this,int param_1)

{
  int iVar1;
  undefined4 local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x53130  32  ??YGSemaphore@@QAEHH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167ad28;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004528a0(&local_14,this + 0x20);
  local_8 = 0;
  while (*(int *)(this + 0x18) < *(int *)(this + 0x1c) + param_1) {
    GWaitCondition::Wait((GWaitCondition *)(this + 0x3c),(GMutex *)(this + 0x20),0xffffffff);
  }
  *(int *)(this + 0x1c) = *(int *)(this + 0x1c) + param_1;
  iVar1 = *(int *)(this + 0x1c);
  local_8 = 0xffffffff;
  FUN_004528d0(&local_14);
  ExceptionList = local_10;
  return iVar1;
}




/* public: int __thiscall GSemaphore::operator-=(int) */

int __thiscall GSemaphore::operator-=(GSemaphore *this,int param_1)

{
  int iVar1;
  cancellation_token_source local_14 [4];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x531d0  33  ??ZGSemaphore@@QAEHH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167acf8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  GMutex::Lock((GMutex *)(this + 0x20));
  if (*(int *)(this + 0x1c) - param_1 < 0) {
    *(undefined4 *)(this + 0x1c) = 0;
  }
  else {
    *(int *)(this + 0x1c) = *(int *)(this + 0x1c) - param_1;
  }
  GWaitCondition::NotifyAll((GWaitCondition *)(this + 0x3c));
  FUN_014babb0((undefined4 *)local_14);
  local_8 = 0;
  FUN_004529c0(this,local_14);
  GMutex::Unlock((GMutex *)(this + 0x20));
  FUN_00452990((undefined4 *)local_14);
  iVar1 = *(int *)(this + 0x1c);
  local_8 = 0xffffffff;
  FUN_004529f0(local_14);
  ExceptionList = local_10;
  return iVar1;
}




/* public: virtual bool __thiscall GSemaphore::CanAcquire(void) */

bool __thiscall GSemaphore::CanAcquire(GSemaphore *this)

{
  int iVar1;
  
                    /* 0x53290  52  ?CanAcquire@GSemaphore@@UAE_NXZ */
  iVar1 = FUN_00452d20((int)(this + -0x14));
  return 0 < iVar1;
}




/* public: virtual bool __thiscall GSemaphore::TryAcquire(void) */

bool __thiscall GSemaphore::TryAcquire(GSemaphore *this)

{
  bool bVar1;
  
                    /* 0x532b0  158  ?TryAcquire@GSemaphore@@UAE_NXZ */
  bVar1 = ObtainSemaphore(this + -0x14,1,0);
  return bVar1;
}




/* public: virtual bool __thiscall GSemaphore::TryAcquireCancel(void) */

bool __thiscall GSemaphore::TryAcquireCancel(GSemaphore *this)

{
  bool bVar1;
  
                    /* 0x532d0  162  ?TryAcquireCancel@GSemaphore@@UAE_NXZ */
  bVar1 = ReleaseSemaphore(this + -0x14,1);
  return bVar1;
}




/* public: virtual bool __thiscall GSemaphore::IsSignaled(void)const  */

bool __thiscall GSemaphore::IsSignaled(GSemaphore *this)

{
  int iVar1;
  
                    /* 0x532f0  101  ?IsSignaled@GSemaphore@@UBE_NXZ */
  iVar1 = FUN_00452d20((int)this);
  return 0 < iVar1;
}




/* public: class GWaitable * __thiscall GSemaphore::CreateWaitableIncrement(int) */

GWaitable * __thiscall GSemaphore::CreateWaitableIncrement(GSemaphore *this,int param_1)

{
  void *this_00;
  GWaitable *local_20;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x53580  62  ?CreateWaitableIncrement@GSemaphore@@QAEPAVGWaitable@@H@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_017903db;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  this_00 = (void *)GFxSprite__unknown_00453600(0x20);
  local_8 = 0;
  if (this_00 == (void *)0x0) {
    local_20 = (GWaitable *)0x0;
  }
  else {
    local_20 = (GWaitable *)FUN_00453330(this_00,this,param_1);
  }
  ExceptionList = local_10;
  return local_20;
}




/* === Additional global functions for GSemaphore (1 functions) === */

/* Also in vtable: GSemaphore (slot 0) */

GSemaphore * __thiscall GSemaphore__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GSemaphore::~GSemaphore(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_(this,0x40,*(int *)((int)this + -4),GSemaphore::~GSemaphore);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== GSemaphoreWaitableIncrement.c ========== */

/*
 * SGW.exe - GSemaphoreWaitableIncrement (2 functions)
 */

/* Also in vtable: GSemaphoreWaitableIncrement (slot 0) */

GRefCountBaseImpl * __thiscall GSemaphoreWaitableIncrement__vfunc_0(void *this,uint param_1)

{
  FUN_00453400(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}


/* [VTable] GSemaphoreWaitableIncrement virtual function #1
   VTable: vtable_GSemaphoreWaitableIncrement at 017fbb68 */

bool __fastcall GSemaphoreWaitableIncrement__vfunc_1(int param_1)

{
  bool bVar1;
  int iVar2;
  
  if (*(int *)(param_1 + 0x18) == 0) {
    bVar1 = false;
  }
  else {
    iVar2 = FUN_00452d20(*(int *)(param_1 + 0x18));
    bVar1 = *(int *)(param_1 + 0x1c) <= iVar2;
  }
  return bVar1;
}




/* ========== GStandardAllocator.c ========== */

/*
 * SGW.exe - GStandardAllocator (7 functions)
 */

/* [VTable] GStandardAllocator virtual function #4
   VTable: vtable_GStandardAllocator at 017fbca4 */

int __fastcall GStandardAllocator__vfunc_4(int param_1)

{
  return param_1 + 0x50c;
}


/* Also in vtable: GStandardAllocator (slot 0) */

CMFCReBar * __thiscall GStandardAllocator__vfunc_0(void *this,uint param_1)

{
  CMFCReBar::~CMFCReBar(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] GStandardAllocator virtual function #3
   VTable: vtable_GStandardAllocator at 017fbca4 */

void * __thiscall GStandardAllocator__vfunc_3(void *this,void *param_1,uint param_2)

{
  uint uVar1;
  undefined4 local_28;
  undefined4 local_24;
  int local_20;
  int local_1c;
  int local_18;
  uint local_14;
  uint local_10;
  uint local_c;
  void *local_8;
  
  local_10 = 0;
  if (param_1 != (void *)0x0) {
    local_10 = *(uint *)((int)param_1 + -4);
  }
  if (param_2 == 0) {
    (**(code **)(*(int *)this + 8))(param_1);
    local_8 = (void *)0x0;
  }
  else {
    if (param_1 == (void *)0x0) {
      local_c = 0;
    }
    else {
      local_20 = (int)param_1 + -4;
      local_1c = local_20;
      local_18 = FUN_00457140(local_20);
      if (local_18 < 0x31) {
        local_c = FUN_00456dc0((int)this + local_18 * 0x14 + 4);
      }
      else {
        local_c = *(int *)(local_1c + -8);
      }
      local_c = local_c - 4;
    }
    if (local_c < param_2) {
      local_14 = local_c;
    }
    else {
      if (local_c >> 1 < param_2) {
        FUN_00451760(&local_24,(int)this + 0x548);
        *(int *)((int)this + 0x514) = *(int *)((int)this + 0x514) + 1;
        if (local_10 < param_2) {
          uVar1 = *(uint *)((int)this + 0x518);
          *(uint *)((int)this + 0x518) = (param_2 - local_10) + *(uint *)((int)this + 0x518);
          *(uint *)((int)this + 0x51c) =
               *(int *)((int)this + 0x51c) + (uint)CARRY4(param_2 - local_10,uVar1);
        }
        else {
          uVar1 = *(uint *)((int)this + 0x520);
          *(uint *)((int)this + 0x520) = (local_10 - param_2) + *(uint *)((int)this + 0x520);
          *(uint *)((int)this + 0x524) =
               *(int *)((int)this + 0x524) + (uint)CARRY4(local_10 - param_2,uVar1);
        }
        *(uint *)((int)param_1 + -4) = (param_2 - local_10) + *(int *)((int)param_1 + -4);
        FUN_00451790(&local_24);
        return param_1;
      }
      local_14 = param_2;
    }
    FUN_00451760(&local_28,(int)this + 0x548);
    *(int *)((int)this + 0x514) = *(int *)((int)this + 0x514) + 1;
    FUN_00451790(&local_28);
    local_8 = (void *)(**(code **)(*(int *)this + 4))(param_2);
    if (local_8 == (void *)0x0) {
      local_8 = (void *)(~-(uint)(local_c < param_2) & (uint)param_1);
    }
    else {
      FUN_004573c0(local_8,param_1,local_14);
      if (local_c != 0) {
        (**(code **)(*(int *)this + 8))(param_1);
      }
    }
  }
  return local_8;
}


/* [VTable] GStandardAllocator virtual function #1
   VTable: vtable_GStandardAllocator at 017fbca4 */

void __thiscall GStandardAllocator__vfunc_1(void *this,int param_1)

{
  FUN_00456880(this,param_1,0,0);
  return;
}


/* [VTable] GStandardAllocator virtual function #2
   VTable: vtable_GStandardAllocator at 017fbca4 */

void __thiscall GStandardAllocator__vfunc_2(void *this,int param_1)

{
  FUN_00456de0(this,param_1,'\0');
  return;
}


/* [VTable] GStandardAllocator virtual function #5
   VTable: vtable_GStandardAllocator at 017fbca4 */

void __thiscall GStandardAllocator__vfunc_5(void *this,int param_1,uint param_2,int param_3)

{
  if (param_2 == 0) {
    param_2 = 4;
  }
  FUN_00456880(this,param_1,param_2,param_3);
  return;
}


/* [VTable] GStandardAllocator virtual function #6
   VTable: vtable_GStandardAllocator at 017fbca4 */

void __thiscall GStandardAllocator__vfunc_6(void *this,int param_1)

{
  FUN_00456de0(this,param_1,'\x01');
  return;
}




/* ========== GStandardBlockAllocator.c ========== */

/*
 * SGW.exe - GStandardBlockAllocator (5 functions)
 */

/* [VTable] GStandardBlockAllocator virtual function #4
   VTable: vtable_GStandardBlockAllocator at 017fbc84 */

int __fastcall GStandardBlockAllocator__vfunc_4(int param_1)

{
  return param_1 + 4;
}


/* Also in vtable: GStandardBlockAllocator (slot 0) */

undefined4 * __thiscall GStandardBlockAllocator__vfunc_0(void *this,uint param_1)

{
  FUN_00455ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] GStandardBlockAllocator virtual function #2
   VTable: vtable_GStandardBlockAllocator at 017fbc84 */

void GStandardBlockAllocator__vfunc_2(LPVOID param_1)

{
  VirtualFree(param_1,0,0x8000);
  return;
}


/* [VTable] GStandardBlockAllocator virtual function #1
   VTable: vtable_GStandardBlockAllocator at 017fbc84 */

void GStandardBlockAllocator__vfunc_1(SIZE_T param_1)

{
  VirtualAlloc((LPVOID)0x0,param_1,0x1000,0x40);
  return;
}


/* [VTable] GStandardBlockAllocator virtual function #3
   VTable: vtable_GStandardBlockAllocator at 017fbc84 */

void GStandardBlockAllocator__vfunc_3(LPVOID param_1,SIZE_T param_2)

{
  VirtualAlloc(param_1,param_2,0x1000,0x40);
  return;
}




/* ========== GSysFile.c ========== */

/*
 * SGW.exe - GSysFile (7 functions)
 */


/* public: virtual bool __thiscall GSysFile::IsValid(void) */

bool __thiscall GSysFile::IsValid(GSysFile *this)

{
  char cVar1;
  int iVar2;
  int *piVar3;
  
                    /* 0xfcdb20  105  ?IsValid@GSysFile@@UAE_NXZ */
  iVar2 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  if (iVar2 != 0) {
    piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    cVar1 = (**(code **)(*piVar3 + 8))();
    if (cVar1 != '\0') {
      return true;
    }
  }
  return false;
}




/* public: __thiscall GSysFile::GSysFile(void) */

GSysFile * __thiscall GSysFile::GSysFile(GSysFile *this)

{
  GRefCountBaseImpl *pGVar1;
  GRefCountBaseImpl *local_20;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
                    /* 0xfceda0  13  ??0GSysFile@@QAE@XZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e293;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_013cda80(this,0);
  local_8 = 0;
  *(undefined ***)this = vftable;
  pGVar1 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x10);
  local_8._0_1_ = 1;
  if (pGVar1 == (GRefCountBaseImpl *)0x0) {
    local_20 = (GRefCountBaseImpl *)0x0;
  }
  else {
    local_20 = FUN_013cee70(pGVar1);
  }
  local_8 = (uint)local_8._1_3_ << 8;
  GFxSprite__unknown_014dedc0(this + 0x10,(int)local_20);
  ExceptionList = local_10;
  return this;
}




/* public: __thiscall GSysFile::GSysFile(char const *,int,int) */

GSysFile * __thiscall GSysFile::GSysFile(GSysFile *this,char *param_1,int param_2,int param_3)

{
  char cVar1;
  void *this_00;
  int iVar2;
  int *piVar3;
  GRefCountBaseImpl *pGVar4;
  GRefCountBaseImpl *local_30;
  GRefCountBaseImpl *local_28;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
                    /* 0xfcef50  12  ??0GSysFile@@QAE@PBDHH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e2ce;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_013cda80(this,0);
  local_8 = 0;
  *(undefined ***)this = vftable;
  this_00 = (void *)GFxSprite__unknown_00453600(0x28);
  local_8._0_1_ = 1;
  if (this_00 == (void *)0x0) {
    local_28 = (GRefCountBaseImpl *)0x0;
  }
  else {
    local_28 = FUN_013cd2a0(this_00,param_1,param_2);
  }
  local_8._0_1_ = 0;
  GFxSprite__unknown_014dedc0(this + 0x10,(int)local_28);
  iVar2 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  if (iVar2 != 0) {
    piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    cVar1 = (**(code **)(*piVar3 + 8))();
    if (cVar1 == '\0') {
      pGVar4 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x10);
      local_8._0_1_ = 2;
      if (pGVar4 == (GRefCountBaseImpl *)0x0) {
        local_30 = (GRefCountBaseImpl *)0x0;
      }
      else {
        local_30 = FUN_013cee70(pGVar4);
      }
      local_8 = (uint)local_8._1_3_ << 8;
      GFxSprite__unknown_014dedc0(this + 0x10,(int)local_30);
    }
  }
  ExceptionList = local_10;
  return this;
}




/* public: bool __thiscall GSysFile::Open(char const *,int,int) */

bool __thiscall GSysFile::Open(GSysFile *this,char *param_1,int param_2,int param_3)

{
  char cVar1;
  void *this_00;
  int iVar2;
  int *piVar3;
  GRefCountBaseImpl *pGVar4;
  GRefCountBaseImpl *local_30;
  GRefCountBaseImpl *local_28;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfcf080  120  ?Open@GSysFile@@QAE_NPBDHH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177e306;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  this_00 = (void *)GFxSprite__unknown_00453600(0x28);
  local_8 = 0;
  if (this_00 == (void *)0x0) {
    local_28 = (GRefCountBaseImpl *)0x0;
  }
  else {
    local_28 = FUN_013cd2a0(this_00,param_1,param_2);
  }
  local_8 = 0xffffffff;
  GFxSprite__unknown_014dedc0(this + 0x10,(int)local_28);
  iVar2 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  if (iVar2 != 0) {
    piVar3 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    cVar1 = (**(code **)(*piVar3 + 8))();
    if (cVar1 != '\0') {
      ExceptionList = local_10;
      return true;
    }
  }
  pGVar4 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x10);
  local_8 = 1;
  if (pGVar4 == (GRefCountBaseImpl *)0x0) {
    local_30 = (GRefCountBaseImpl *)0x0;
  }
  else {
    local_30 = FUN_013cee70(pGVar4);
  }
  local_8 = 0xffffffff;
  GFxSprite__unknown_014dedc0(this + 0x10,(int)local_30);
  ExceptionList = local_10;
  return false;
}




/* public: static bool __cdecl GSysFile::GetFileStat(struct GFileStat *,char const *) */

bool __cdecl GSysFile::GetFileStat(GFileStat *param_1,char *param_2)

{
  int iVar1;
  undefined1 local_3c [24];
  undefined4 local_24;
  undefined4 local_20;
  undefined4 local_1c;
  undefined4 local_18;
  undefined4 local_14;
  undefined4 local_10;
  
                    /* 0xfcf190  83  ?GetFileStat@GSysFile@@SA_NPAUGFileStat@@PBD@Z */
  iVar1 = stat64(param_2,local_3c);
  if (iVar1 == 0) {
    *(undefined4 *)(param_1 + 8) = local_1c;
    *(undefined4 *)(param_1 + 0xc) = local_18;
    *(undefined4 *)param_1 = local_14;
    *(undefined4 *)(param_1 + 4) = local_10;
    *(undefined4 *)(param_1 + 0x10) = local_24;
    *(undefined4 *)(param_1 + 0x14) = local_20;
  }
  return iVar1 == 0;
}




/* public: virtual int __thiscall GSysFile::GetErrorCode(void) */

int __thiscall GSysFile::GetErrorCode(GSysFile *this)

{
  int iVar1;
  int *piVar2;
  int local_10;
  
                    /* 0xfcf1f0  79  ?GetErrorCode@GSysFile@@UAEHXZ */
  iVar1 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
  if (iVar1 == 0) {
    local_10 = 0x1001;
  }
  else {
    piVar2 = (int *)GFxSprite__unknown_014b3780((undefined4 *)(this + 0x10));
    local_10 = (**(code **)(*piVar2 + 0x20))();
  }
  return local_10;
}




/* public: virtual bool __thiscall GSysFile::Close(void) */

bool __thiscall GSysFile::Close(GSysFile *this)

{
  char cVar1;
  bool bVar2;
  GRefCountBaseImpl *pGVar3;
  GRefCountBaseImpl *local_20;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfcf240  56  ?Close@GSysFile@@UAE_NXZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_017903db;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  cVar1 = (**(code **)(*(int *)this + 8))(DAT_01e7f9a0 ^ (uint)&stack0xfffffffc);
  if (cVar1 == '\0') {
    bVar2 = false;
  }
  else {
    FUN_013cdea0((int)this);
    pGVar3 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x10);
    local_8 = 0;
    if (pGVar3 == (GRefCountBaseImpl *)0x0) {
      local_20 = (GRefCountBaseImpl *)0x0;
    }
    else {
      local_20 = FUN_013cee70(pGVar3);
    }
    local_8 = 0xffffffff;
    GFxSprite__unknown_014dedc0(this + 0x10,(int)local_20);
    bVar2 = true;
  }
  ExceptionList = local_10;
  return bVar2;
}




/* === Additional global functions for GSysFile (1 functions) === */

GRefCountBaseImpl * __thiscall GSysFile__vfunc_0(void *this,uint param_1)

{
  FUN_013c8ce0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GTexture.c ========== */

/*
 * SGW.exe - GTexture (2 functions)
 */

/* [VTable] GTexture virtual function #13
   VTable: vtable_GTexture at 019611a0 */

void __thiscall GTexture__vfunc_13(int *param_1,undefined4 param_2,undefined4 param_3,void *param_4)

{
  undefined1 uStack00000014;
  uint3 uStack00000015;
  
  *param_1 = (int)UActorFactory::vftable;
  _uStack00000014 = 2;
  param_2 = param_1;
  FUN_0049f140(param_1);
  uStack00000015 = (uint3)((uint)_uStack00000014 >> 8);
  uStack00000014 = 1;
  FUN_0041b420(param_1 + 0x16);
  _uStack00000014 = (uint)uStack00000015 << 8;
  FUN_0041b420(param_1 + 0x10);
  _uStack00000014 = 0xffffffff;
  FUN_004a0dc0(param_1);
  ExceptionList = param_4;
  return;
}


/* Also in vtable: GTexture (slot 0) */

undefined4 * __thiscall GTexture__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = GTexture::vftable;
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  return this;
}




/* ========== GTexture_ChangeHandler.c ========== */

/*
 * SGW.exe - GTexture_ChangeHandler (1 functions)
 */

/* Also in vtable: GTexture_ChangeHandler (slot 0) */

undefined4 * __thiscall GTexture_ChangeHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = GTexture::ChangeHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== GThread.c ========== */

/*
 * SGW.exe - GThread (19 functions)
 */


/* public: void __thiscall GThread::`default constructor closure'(void) */

void __thiscall GThread::_default_constructor_closure_(GThread *this)

{
                    /* 0x1cef0  37  ??_FGThread@@QAEXXZ */
  GThread(this,0x20000,-1);
  return;
}




/* public: __thiscall GThread::GThread(unsigned int,int) */

GThread * __thiscall GThread::GThread(GThread *this,uint param_1,int param_2)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x54700  14  ??0GThread@@QAE@IH@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aea3;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004514d0(this,'\x01');
  local_8 = 0;
  FUN_00451db0((undefined4 *)(this + 0x14));
  local_8 = CONCAT31(local_8._1_3_,1);
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  FUN_00453670((IUnknown *)(this + 0x18));
  FUN_00453670((IUnknown *)(this + 0x1c));
  FUN_01407080(this + 0x18,0);
  *(undefined4 *)(this + 0x24) = 0;
  *(undefined4 *)(this + 0x28) = 0;
  FUN_01407080(this + 0x1c,0);
  *(uint *)(this + 0x20) = param_1;
  *(int *)(this + 0x2c) = param_2;
  *(undefined4 *)(this + 0x30) = 0;
  *(undefined4 *)(this + 0x34) = 0;
  ExceptionList = local_10;
  return this;
}




/* public: bool __thiscall GThread::IsFinished(void)const  */

bool __thiscall GThread::IsFinished(GThread *this)

{
  uint uVar1;
  
                    /* 0x547e0  97  ?IsFinished@GThread@@QBE_NXZ */
  uVar1 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x18));
  return (uVar1 & 2) != 0;
}




/* public: __thiscall GThread::GThread(int (__cdecl*)(class GThread *,void *),void *,unsigned
   int,int,enum GThread::ThreadState) */

GThread * __thiscall
GThread::GThread(GThread *this,_func_int_GThread_ptr_void_ptr *param_1,void *param_2,uint param_3,
                int param_4,ThreadState param_5)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x548a0  15  ??0GThread@@QAE@P6AHPAV0@PAX@Z1IHW4ThreadState@0@@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aea3;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_004514d0(this,'\x01');
  local_8 = 0;
  FUN_00451db0((undefined4 *)(this + 0x14));
  local_8 = CONCAT31(local_8._1_3_,1);
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  FUN_00453670((IUnknown *)(this + 0x18));
  FUN_00453670((IUnknown *)(this + 0x1c));
  FUN_01407080(this + 0x18,0);
  *(undefined4 *)(this + 0x24) = 0;
  *(undefined4 *)(this + 0x28) = 0;
  FUN_01407080(this + 0x1c,0);
  *(uint *)(this + 0x20) = param_3;
  *(int *)(this + 0x2c) = param_4;
  *(_func_int_GThread_ptr_void_ptr **)(this + 0x30) = param_1;
  *(void **)(this + 0x34) = param_2;
  if (param_5 != 0) {
    Start(this,param_5);
  }
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GThread::~GThread(void) */

void __thiscall GThread::~GThread(GThread *this)

{
  GAcquireInterface *local_18;
  void *local_10;
  undefined1 *puStack_c;
  uint local_8;
  
                    /* 0x54990  27  ??1GThread@@UAE@XZ */
  puStack_c = &LAB_0167ad7f;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  *(undefined ***)(this + 0x14) = vftable;
  local_8 = 1;
  FUN_00455250((int)this);
  *(undefined4 *)(this + 0x24) = 0;
  local_8 = local_8 & 0xffffff00;
  if (this == (GThread *)0x0) {
    local_18 = (GAcquireInterface *)0x0;
  }
  else {
    local_18 = (GAcquireInterface *)(this + 0x14);
  }
  GAcquireInterface::~GAcquireInterface(local_18);
  local_8 = 0xffffffff;
  FUN_00451650((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* public: virtual int __thiscall GThread::Run(void) */

int __thiscall GThread::Run(GThread *this)

{
  int local_c;
  
                    /* 0x54a30  130  ?Run@GThread@@UAEHXZ */
  if (*(int *)(this + 0x30) == 0) {
    local_c = 0;
  }
  else {
    local_c = (**(code **)(this + 0x30))(this,*(undefined4 *)(this + 0x34));
  }
  return local_c;
}




/* public: static void __cdecl GThread::FinishAllThreads(void) */

void __cdecl GThread::FinishAllThreads(void)

{
                    /* 0x54b20  66  ?FinishAllThreads@GThread@@SAXXZ */
  FUN_00454b30();
  return;
}




/* public: bool __thiscall GThread::GetExitFlag(void)const  */

bool __thiscall GThread::GetExitFlag(GThread *this)

{
  uint uVar1;
  
                    /* 0x54d00  81  ?GetExitFlag@GThread@@QBE_NXZ */
  uVar1 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x18));
  return (uVar1 & 0x10) != 0;
}




/* public: void __thiscall GThread::SetExitFlag(bool) */

void __thiscall GThread::SetExitFlag(GThread *this,bool param_1)

{
                    /* 0x54d20  135  ?SetExitFlag@GThread@@QAEX_N@Z */
  if (param_1) {
    FUN_00455320(this + 0x18,0x10);
  }
  else {
    FUN_004552e0(this + 0x18,0xffffffef);
  }
  return;
}




/* public: bool __thiscall GThread::IsSuspended(void)const  */

bool __thiscall GThread::IsSuspended(GThread *this)

{
  int iVar1;
  
                    /* 0x54d60  103  ?IsSuspended@GThread@@QBE_NXZ */
  iVar1 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x1c));
  return 0 < iVar1;
}




/* public: enum GThread::ThreadState __thiscall GThread::GetThreadState(void)const  */

ThreadState __thiscall GThread::GetThreadState(GThread *this)

{
  bool bVar1;
  ThreadState TVar2;
  uint uVar3;
  
                    /* 0x54d80  92  ?GetThreadState@GThread@@QBE?AW4ThreadState@1@XZ */
  bVar1 = IsSuspended(this);
  if (bVar1) {
    TVar2 = 2;
  }
  else {
    uVar3 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x18));
    if ((uVar3 & 1) == 0) {
      TVar2 = 0;
    }
    else {
      TVar2 = 1;
    }
  }
  return TVar2;
}




/* public: virtual bool __thiscall GThread::Start(enum GThread::ThreadState) */

bool __thiscall GThread::Start(GThread *this,ThreadState param_1)

{
  bool bVar1;
  ThreadState TVar2;
  uintptr_t uVar3;
  
                    /* 0x54ed0  147  ?Start@GThread@@UAE_NW4ThreadState@1@@Z */
  if (param_1 == 0) {
    bVar1 = false;
  }
  else {
    TVar2 = GetThreadState(this);
    if ((TVar2 != 0) && (bVar1 = GWaitable::Wait((GWaitable *)this,0xffffffff), !bVar1)) {
      return false;
    }
    FUN_00455250((int)this);
    FUN_00454fc0((int)this);
    FUN_00454fe0();
    *(undefined4 *)(this + 0x28) = 0;
    FUN_01407080(this + 0x1c,0);
    FUN_01407080(this + 0x18,-(uint)(param_1 != 1) & 8);
    uVar3 = _beginthreadex((void *)0x0,*(uint *)(this + 0x20),FUN_00454dc0,this,0,(uint *)0x0);
    *(uintptr_t *)(this + 0x24) = uVar3;
    if (*(int *)(this + 0x24) == 0) {
      FUN_01407080(this + 0x18,0);
      FUN_00454b00((int)this);
      FUN_00454e20();
      bVar1 = false;
    }
    else {
      bVar1 = true;
    }
  }
  return bVar1;
}




/* public: bool __thiscall GThread::Suspend(void) */

bool __thiscall GThread::Suspend(GThread *this)

{
  bool bVar1;
  uint uVar2;
  DWORD DVar3;
  
                    /* 0x55160  148  ?Suspend@GThread@@QAE_NXZ */
  uVar2 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x18));
  if ((uVar2 & 1) == 0) {
    bVar1 = false;
  }
  else {
    DVar3 = SuspendThread(*(HANDLE *)(this + 0x24));
    if (DVar3 == 0xffffffff) {
      bVar1 = false;
    }
    else {
      FUN_004537f0(this + 0x1c);
      bVar1 = true;
    }
  }
  return bVar1;
}




/* public: bool __thiscall GThread::Resume(void) */

bool __thiscall GThread::Resume(GThread *this)

{
  uint uVar1;
  int iVar2;
  DWORD DVar3;
  
                    /* 0x551b0  129  ?Resume@GThread@@QAE_NXZ */
  uVar1 = GFxSprite__unknown_014b3780((undefined4 *)(this + 0x18));
  if ((uVar1 & 1) == 0) {
    return false;
  }
  iVar2 = FUN_00453810(this + 0x1c,-1);
  if (0 < iVar2) {
    if (iVar2 != 1) {
      return true;
    }
    DVar3 = ResumeThread(*(HANDLE *)(this + 0x24));
    if (DVar3 != 0xffffffff) {
      return true;
    }
  }
  return false;
}




/* public: virtual void __thiscall GThread::Exit(int) */

void __thiscall GThread::Exit(GThread *this,int param_1)

{
                    /* 0x55210  65  ?Exit@GThread@@UAEXH@Z */
  (**(code **)(*(int *)this + 0x10))();
  FUN_00454a70(this);
  FUN_00454e20();
  _endthreadex(param_1);
  return;
}




/* public: virtual bool __thiscall GThread::Sleep(unsigned int) */

bool __thiscall GThread::Sleep(GThread *this,uint param_1)

{
                    /* 0x55280  146  ?Sleep@GThread@@UAE_NI@Z */
  ::Sleep(param_1 * 1000);
  return true;
}




/* public: virtual bool __thiscall GThread::MSleep(unsigned int) */

bool __thiscall GThread::MSleep(GThread *this,uint param_1)

{
                    /* 0x552a0  115  ?MSleep@GThread@@UAE_NI@Z */
  ::Sleep(param_1);
  return true;
}




/* public: static int __cdecl GThread::GetCPUCount(void) */

int __cdecl GThread::GetCPUCount(void)

{
  _SYSTEM_INFO local_28;
  
                    /* 0x552c0  76  ?GetCPUCount@GThread@@SAHXZ */
  GetSystemInfo(&local_28);
  return local_28.dwNumberOfProcessors;
}




/* public: virtual void __thiscall GThread::OnExit(void)
   Also in vtable: GSemaphoreWaitableIncrement (slot 3)
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 5) */

void __thiscall GThread::OnExit(GThread *this)

{
                    /* 0x10903b0  119  ?OnExit@GThread@@UAEXXZ */
  return;
}




/* === Additional global functions for GThread (1 functions) === */

/* Also in vtable: GThread (slot 0) */

GThread * __thiscall GThread__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GThread::~GThread(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_(this,0x38,*(int *)((int)this + -4),GThread::~GThread);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== GUnopenedFile.c ========== */

/*
 * SGW.exe - GUnopenedFile (1 functions)
 */

GRefCountBaseImpl * __thiscall GUnopenedFile__vfunc_0(void *this,uint param_1)

{
  FUN_013cef30(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GWaitCondition.c ========== */

/*
 * SGW.exe - GWaitCondition (5 functions)
 */


/* public: __thiscall GWaitCondition::GWaitCondition(void) */

GWaitCondition * __thiscall GWaitCondition::GWaitCondition(GWaitCondition *this)

{
  void *pvVar1;
  void *local_20;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x545b0  16  ??0GWaitCondition@@QAE@XZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_017903db;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  pvVar1 = (void *)GFxSprite__unknown_0146a300(0x20);
  local_8 = 0;
  if (pvVar1 == (void *)0x0) {
    local_20 = (void *)0x0;
  }
  else {
    local_20 = FUN_00454040(pvVar1);
  }
  *(void **)this = local_20;
  ExceptionList = local_10;
  return this;
}




/* public: __thiscall GWaitCondition::~GWaitCondition(void) */

void __thiscall GWaitCondition::~GWaitCondition(GWaitCondition *this)

{
                    /* 0x54630  28  ??1GWaitCondition@@QAE@XZ */
  if (*(void **)this != (void *)0x0) {
    FUN_00454670(*(void **)this,1);
  }
  return;
}




/* public: bool __thiscall GWaitCondition::Wait(class GMutex *,unsigned int) */

bool __thiscall GWaitCondition::Wait(GWaitCondition *this,GMutex *param_1,uint param_2)

{
  undefined1 uVar1;
  
                    /* 0x546a0  170  ?Wait@GWaitCondition@@QAE_NPAVGMutex@@I@Z */
  uVar1 = FUN_00454330(*(void **)this,(GWaitable *)param_1,param_2);
  return (bool)uVar1;
}




/* public: void __thiscall GWaitCondition::Notify(void) */

void __thiscall GWaitCondition::Notify(GWaitCondition *this)

{
                    /* 0x546c0  116  ?Notify@GWaitCondition@@QAEXXZ */
  FUN_004544a0(*(int *)this);
  return;
}




/* public: void __thiscall GWaitCondition::NotifyAll(void) */

void __thiscall GWaitCondition::NotifyAll(GWaitCondition *this)

{
                    /* 0x546e0  117  ?NotifyAll@GWaitCondition@@QAEXXZ */
  FUN_00454520(*(int *)this);
  return;
}





/* ========== GWaitable.c ========== */

/*
 * SGW.exe - GWaitable (8 functions)
 */


/* public: bool __thiscall GWaitable::AddWaitHandler(void (__cdecl*)(void *),void *) */

bool __thiscall
GWaitable::AddWaitHandler(GWaitable *this,_func_void_void_ptr *param_1,void *param_2)

{
  int iVar1;
  undefined4 local_1c;
  undefined4 local_18 [2];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x516c0  42  ?AddWaitHandler@GWaitable@@QAE_NP6AXPAX@Z0@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167a9f8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  iVar1 = *(int *)(this + 0x10);
  if (iVar1 != 0) {
    FUN_013cafc0(local_18,param_1,param_2);
    FUN_00451760(&local_1c,*(int *)(this + 0x10) + 0x10);
    local_8 = 0;
    FUN_004536b0((void *)(*(int *)(this + 0x10) + 4),local_18);
    local_8 = 0xffffffff;
    FUN_00451790(&local_1c);
  }
  ExceptionList = local_10;
  return iVar1 != 0;
}




/* public: bool __thiscall GWaitable::RemoveWaitHandler(void (__cdecl*)(void *),void *) */

bool __thiscall
GWaitable::RemoveWaitHandler(GWaitable *this,_func_void_void_ptr *param_1,void *param_2)

{
  char cVar1;
  uint uVar2;
  void *this_00;
  int *piVar3;
  uint local_24;
  bool local_1d;
  undefined4 local_1c;
  int local_18 [2];
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x517b0  127  ?RemoveWaitHandler@GWaitable@@QAE_NP6AXPAX@Z0@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aa28;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  if (*(int *)(this + 0x10) == 0) {
    local_1d = false;
  }
  else {
    FUN_013cafc0(local_18,param_1,param_2);
    local_1d = false;
    FUN_00451760(&local_1c,*(int *)(this + 0x10) + 0x10);
    local_8 = 0;
    local_24 = 0;
    while( true ) {
      uVar2 = GSemaphoreWaitableIncrement__unknown_0144af00(*(int *)(this + 0x10) + 4);
      if (uVar2 <= local_24) break;
      piVar3 = local_18;
      this_00 = (void *)GFxSprite__unknown_014296f0((void *)(*(int *)(this + 0x10) + 4),local_24);
      cVar1 = FUN_004518b0(this_00,piVar3);
      if (cVar1 != '\0') {
        FUN_00453700((void *)(*(int *)(this + 0x10) + 4),local_24);
        local_1d = true;
        break;
      }
      local_24 = local_24 + 1;
    }
    local_8 = 0xffffffff;
    FUN_00451790(&local_1c);
  }
  ExceptionList = local_10;
  return local_1d;
}




/* public: void __thiscall GWaitable::HandlerArray::Release(void) */

void __thiscall GWaitable::HandlerArray::Release(HandlerArray *this)

{
  int iVar1;
  
                    /* 0x518f0  125  ?Release@HandlerArray@GWaitable@@QAEXXZ */
  iVar1 = FUN_00456020(this,-1);
  if ((iVar1 == 1) && (this != (HandlerArray *)0x0)) {
    FUN_00451940(this,1);
  }
  return;
}




/* public: void __thiscall GWaitable::HandlerArray::CallWaitHandlers(void) */

void __thiscall GWaitable::HandlerArray::CallWaitHandlers(HandlerArray *this)

{
  undefined4 *puVar1;
  int iVar2;
  uint uVar3;
  undefined4 uVar4;
  uint local_30;
  int local_2c [3];
  undefined1 local_20 [8];
  int local_18;
  undefined4 local_14;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
                    /* 0x519d0  48  ?CallWaitHandlers@HandlerArray@GWaitable@@QAEXXZ */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aa60;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_00451760(&local_14,this + 0x10);
  local_8 = 0;
  local_18 = GSemaphoreWaitableIncrement__unknown_0144af00((int)(this + 4));
  if (local_18 == 0) {
    local_8 = 0xffffffff;
    FUN_00451790(&local_14);
  }
  else {
    if (local_18 == 1) {
      puVar1 = (undefined4 *)GFxSprite__unknown_014296f0(this + 4,0);
      FUN_01496440(local_20,puVar1);
      iVar2 = GFxSprite__unknown_014296f0(this + 4,0);
      uVar4 = *(undefined4 *)(iVar2 + 4);
      puVar1 = (undefined4 *)GFxSprite__unknown_014296f0(this + 4,0);
      (*(code *)*puVar1)(uVar4);
    }
    else {
      FUN_00453690(local_2c,this + 4);
      local_8._0_1_ = 1;
      local_30 = 0;
      while( true ) {
        uVar3 = GSemaphoreWaitableIncrement__unknown_0144af00((int)local_2c);
        if (uVar3 <= local_30) break;
        iVar2 = GFxSprite__unknown_014296f0(local_2c,local_30);
        uVar4 = *(undefined4 *)(iVar2 + 4);
        puVar1 = (undefined4 *)GFxSprite__unknown_014296f0(local_2c,local_30);
        (*(code *)*puVar1)(uVar4);
        local_30 = local_30 + 1;
      }
      local_8 = (uint)local_8._1_3_ << 8;
      FUN_0140d8c0(local_2c);
    }
    local_8 = 0xffffffff;
    FUN_00451790(&local_14);
  }
  ExceptionList = local_10;
  return;
}




/* public: void __thiscall GWaitable::CallWaitHandlers(void) */

void __thiscall GWaitable::CallWaitHandlers(GWaitable *this)

{
                    /* 0x51b00  47  ?CallWaitHandlers@GWaitable@@QAEXXZ */
  if (*(int *)(this + 0x10) != 0) {
    HandlerArray::CallWaitHandlers(*(HandlerArray **)(this + 0x10));
  }
  return;
}




/* public: bool __thiscall GWaitable::Acquire(unsigned int) */

bool __thiscall GWaitable::Acquire(GWaitable *this,uint param_1)

{
  int iVar1;
  GWaitable *local_8;
  
                    /* 0x51b20  39  ?Acquire@GWaitable@@QAE_NI@Z */
  local_8 = this;
  iVar1 = GAcquireInterface::AcquireOneOfMultipleObjects(&local_8,1,param_1);
  return (bool)('\x01' - (iVar1 != 0));
}




/* public: bool __thiscall GWaitable::Wait(unsigned int) */

bool __thiscall GWaitable::Wait(GWaitable *this,uint param_1)

{
  char cVar1;
  bool bVar2;
  undefined1 uVar3;
  int iVar4;
  undefined4 extraout_EDX;
  undefined1 local_68 [8];
  int local_60;
  undefined4 local_5c;
  undefined1 local_51;
  GEvent local_50 [60];
  uint local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0x51b90  171  ?Wait@GWaitable@@QAE_NI@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0167aa88;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  cVar1 = (**(code **)(*(int *)this + 4))(DAT_01e7f9a0 ^ (uint)&stack0xfffffffc);
  if (cVar1 == '\0') {
    if (param_1 == 0) {
      uVar3 = 0;
    }
    else {
      GEvent::GEvent(local_50,false,false);
      local_8 = 0;
      FUN_013cafc0(local_68,this,local_50);
      bVar2 = AddWaitHandler(this,FUN_00451b50,local_68);
      if (bVar2) {
        cVar1 = (**(code **)(*(int *)this + 4))();
        if (cVar1 == '\0') {
          local_51 = 0;
          local_60 = 0;
          local_5c = 0;
          local_14 = param_1;
          if (param_1 != 0xffffffff) {
            local_60 = FUN_004560c0();
            local_5c = extraout_EDX;
          }
          while (bVar2 = GEvent::Wait(local_50,local_14), bVar2) {
            cVar1 = (**(code **)(*(int *)this + 4))();
            if (cVar1 != '\0') {
              local_51 = 1;
              break;
            }
            if (param_1 != 0xffffffff) {
              iVar4 = FUN_004560c0();
              if (param_1 <= (uint)(iVar4 - local_60)) break;
              local_14 = param_1 - (iVar4 - local_60);
            }
          }
          RemoveWaitHandler(this,FUN_00451b50,local_68);
          uVar3 = local_51;
          local_8 = 0xffffffff;
          GEvent::~GEvent(local_50);
        }
        else {
          RemoveWaitHandler(this,FUN_00451b50,local_68);
          local_8 = 0xffffffff;
          GEvent::~GEvent(local_50);
          uVar3 = 1;
        }
      }
      else {
        local_8 = 0xffffffff;
        GEvent::~GEvent(local_50);
        uVar3 = 0;
      }
    }
  }
  else {
    uVar3 = 1;
  }
  ExceptionList = local_10;
  return (bool)uVar3;
}




/* public: virtual class GAcquireInterface * __thiscall GWaitable::GetAcquireInterface(void) */

GAcquireInterface * __thiscall GWaitable::GetAcquireInterface(GWaitable *this)

{
  GDefaultAcquireInterface *pGVar1;
  
                    /* 0x51d40  75  ?GetAcquireInterface@GWaitable@@UAEPAVGAcquireInterface@@XZ */
  pGVar1 = GDefaultAcquireInterface::GetDefaultAcquireInterface();
  return (GAcquireInterface *)pGVar1;
}




/* === Additional global functions for GWaitable (1 functions) === */

/* Also in vtable: GWaitable (slot 0) */

GRefCountBaseImpl * __thiscall GWaitable__vfunc_0(void *this,uint param_1)

{
  FUN_00451650(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== GZLibFile.c ========== */

/*
 * SGW.exe - GZLibFile (18 functions)
 */


/* public: void __thiscall GZLibFile::`default constructor closure'(void) */

void __thiscall GZLibFile::_default_constructor_closure_(GZLibFile *this)

{
                    /* 0xfd5b20  38  ??_FGZLibFile@@QAEXXZ */
  GZLibFile(this,(GFile *)0x0);
  return;
}




/* public: __thiscall GZLibFile::GZLibFile(class GFile *) */

GZLibFile * __thiscall GZLibFile::GZLibFile(GZLibFile *this,GFile *param_1)

{
  char cVar1;
  uint uVar2;
  void *this_00;
  undefined4 *local_20;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfe3150  17  ??0GZLibFile@@QAE@PAVGFile@@@Z */
  local_8 = 0xffffffff;
  puStack_c = &LAB_0177f583;
  local_10 = ExceptionList;
  uVar2 = DAT_01e7f9a0 ^ (uint)&stack0xfffffffc;
  ExceptionList = &local_10;
  FUN_00aff470((GRefCountBaseImpl *)this);
  local_8 = 0;
  *(undefined ***)this = vftable;
  *(undefined4 *)(this + 0x10) = 0;
  if (param_1 != (GFile *)0x0) {
    cVar1 = (**(code **)(*(int *)param_1 + 8))(uVar2);
    if (cVar1 != '\0') {
      this_00 = (void *)GFxSprite__unknown_0146a300(0x1458);
      local_8 = CONCAT31(local_8._1_3_,1);
      if (this_00 == (void *)0x0) {
        local_20 = (undefined4 *)0x0;
      }
      else {
        local_20 = FUN_013e3290(this_00,(int)param_1);
      }
      *(undefined4 **)(this + 0x10) = local_20;
    }
  }
  ExceptionList = local_10;
  return this;
}




/* public: virtual __thiscall GZLibFile::~GZLibFile(void) */

void __thiscall GZLibFile::~GZLibFile(GZLibFile *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
                    /* 0xfe33c0  29  ??1GZLibFile@@UAE@XZ */
  puStack_c = &LAB_0177f5d8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  *(undefined ***)this = vftable;
  local_8 = 0;
  if (*(int *)(this + 0x10) != 0) {
    FUN_013e3470(*(undefined4 **)(this + 0x10));
    FUN_012a33f0(*(int *)(this + 0x10) + 4);
    if (*(void **)(this + 0x10) != (void *)0x0) {
      FUN_013e34d0(*(void **)(this + 0x10),1);
    }
  }
  local_8 = 0xffffffff;
  FUN_00aff230((GRefCountBaseImpl *)this);
  ExceptionList = local_10;
  return;
}




/* public: virtual char const * __thiscall GZLibFile::GetFilePath(void) */

char * __thiscall GZLibFile::GetFilePath(GZLibFile *this)

{
  int *piVar1;
  char *local_10;
  
                    /* 0xfe3500  82  ?GetFilePath@GZLibFile@@UAEPBDXZ */
  if (*(int *)(this + 0x10) == 0) {
    local_10 = (char *)0x0;
  }
  else {
    piVar1 = (int *)GFxSprite__unknown_014b3780(*(undefined4 **)(this + 0x10));
    local_10 = (char *)(**(code **)(*piVar1 + 4))();
  }
  return local_10;
}




/* public: virtual bool __thiscall GZLibFile::IsValid(void) */

bool __thiscall GZLibFile::IsValid(GZLibFile *this)

{
                    /* 0xfe3540  106  ?IsValid@GZLibFile@@UAE_NXZ */
  return *(int *)(this + 0x10) != 0;
}




/* public: virtual int __thiscall GZLibFile::Tell(void) */

int __thiscall GZLibFile::Tell(GZLibFile *this)

{
  int iVar1;
  
                    /* 0xfe3560  151  ?Tell@GZLibFile@@UAEHXZ */
  if (*(int *)(this + 0x10) == 0) {
    iVar1 = -1;
  }
  else {
    iVar1 = *(int *)(*(int *)(this + 0x10) + 0x4c);
  }
  return iVar1;
}




/* public: virtual __int64 __thiscall GZLibFile::LTell(void) */

__int64 __thiscall GZLibFile::LTell(GZLibFile *this)

{
  int iVar1;
  
                    /* 0xfe3590  112  ?LTell@GZLibFile@@UAE_JXZ */
  iVar1 = (**(code **)(*(int *)this + 0x10))();
  return (__int64)iVar1;
}




/* public: virtual int __thiscall GZLibFile::GetLength(void) */

int __thiscall GZLibFile::GetLength(GZLibFile *this)

{
  undefined4 uVar1;
  int iVar2;
  
                    /* 0xfe35b0  89  ?GetLength@GZLibFile@@UAEHXZ */
  if (*(int *)(this + 0x10) == 0) {
    iVar2 = 0;
  }
  else if (*(int *)(*(int *)(this + 0x10) + 0x48) == 0) {
    uVar1 = *(undefined4 *)(*(int *)(this + 0x10) + 0x4c);
    iVar2 = FUN_013e3610((int *)this);
    (**(code **)(*(int *)this + 0x38))(uVar1,0);
  }
  else {
    iVar2 = 0;
  }
  return iVar2;
}




/* public: virtual __int64 __thiscall GZLibFile::LGetLength(void) */

__int64 __thiscall GZLibFile::LGetLength(GZLibFile *this)

{
  int iVar1;
  
                    /* 0xfe3630  108  ?LGetLength@GZLibFile@@UAE_JXZ */
  iVar1 = (**(code **)(*(int *)this + 0x18))();
  return (__int64)iVar1;
}




/* public: virtual int __thiscall GZLibFile::GetErrorCode(void) */

int __thiscall GZLibFile::GetErrorCode(GZLibFile *this)

{
  int local_c;
  
                    /* 0xfe3650  80  ?GetErrorCode@GZLibFile@@UAEHXZ */
  if (*(int *)(this + 0x10) == 0) {
    local_c = 0;
  }
  else {
    local_c = *(int *)(*(int *)(this + 0x10) + 0x48);
  }
  return local_c;
}




/* public: virtual int __thiscall GZLibFile::CopyFromStream(class GFile *,int) */

int __thiscall GZLibFile::CopyFromStream(GZLibFile *this,GFile *param_1,int param_2)

{
                    /* 0xfe3680  61  ?CopyFromStream@GZLibFile@@UAEHPAVGFile@@H@Z
                       0xfe3680  173  ?Write@GZLibFile@@UAEHPBEH@Z */
  return 0;
}




/* public: virtual int __thiscall GZLibFile::Read(unsigned char *,int) */

int __thiscall GZLibFile::Read(GZLibFile *this,uchar *param_1,int param_2)

{
  size_t sVar1;
  
                    /* 0xfe3690  124  ?Read@GZLibFile@@UAEHPAEH@Z */
  if (*(int *)(this + 0x10) == 0) {
    sVar1 = 0xffffffff;
  }
  else {
    sVar1 = FUN_013e36c0(*(void **)(this + 0x10),param_1,param_2);
  }
  return sVar1;
}




/* public: virtual int __thiscall GZLibFile::SkipBytes(int) */

int __thiscall GZLibFile::SkipBytes(GZLibFile *this,int param_1)

{
  int iVar1;
  
                    /* 0xfe3a30  145  ?SkipBytes@GZLibFile@@UAEHH@Z */
  iVar1 = (**(code **)(*(int *)this + 0x38))(param_1,1);
  return iVar1;
}




/* public: virtual int __thiscall GZLibFile::BytesAvailable(void) */

int __thiscall GZLibFile::BytesAvailable(GZLibFile *this)

{
  int iVar1;
  int iVar2;
  
                    /* 0xfe3a50  46  ?BytesAvailable@GZLibFile@@UAEHXZ */
  if (*(int *)(this + 0x10) == 0) {
    iVar2 = 0;
  }
  else if (*(int *)(*(int *)(this + 0x10) + 0x48) == 0) {
    iVar1 = *(int *)(*(int *)(this + 0x10) + 0x4c);
    iVar2 = FUN_013e3610((int *)this);
    (**(code **)(*(int *)this + 0x38))(iVar1,0);
    iVar2 = iVar2 - iVar1;
  }
  else {
    iVar2 = 0;
  }
  return iVar2;
}




/* public: virtual int __thiscall GZLibFile::Seek(int,int) */

int __thiscall GZLibFile::Seek(GZLibFile *this,int param_1,int param_2)

{
                    /* 0xfe3ab0  132  ?Seek@GZLibFile@@UAEHHH@Z */
  if (*(int *)(this + 0x10) == 0) {
    return -1;
  }
  if (*(int *)(*(int *)(this + 0x10) + 0x48) != 0) {
    return *(int *)(*(int *)(this + 0x10) + 0x4c);
  }
  if (param_2 != 0) {
    if (param_2 == 1) {
      param_1 = param_1 + *(int *)(*(int *)(this + 0x10) + 0x4c);
    }
    else {
      if ((param_2 != 2) || (FUN_013e3b60(*(void **)(this + 0x10),0x7fffffff), param_1 == 0))
      goto LAB_013e3b42;
      param_1 = *(int *)(*(int *)(this + 0x10) + 0x4c) + param_1;
    }
  }
  FUN_013e3b60(*(void **)(this + 0x10),param_1);
LAB_013e3b42:
  return *(int *)(*(int *)(this + 0x10) + 0x4c);
}




/* public: virtual __int64 __thiscall GZLibFile::LSeek(__int64,int) */

__int64 __thiscall GZLibFile::LSeek(GZLibFile *this,__int64 param_1,int param_2)

{
  int iVar1;
  
                    /* 0xfe3d30  110  ?LSeek@GZLibFile@@UAE_J_JH@Z */
  iVar1 = (**(code **)(*(int *)this + 0x38))((undefined4)param_1,param_2);
  return (__int64)iVar1;
}




/* public: virtual bool __thiscall GZLibFile::Close(void) */

bool __thiscall GZLibFile::Close(GZLibFile *this)

{
  int iVar1;
  int *piVar2;
  bool bVar3;
  
                    /* 0xfe3d60  57  ?Close@GZLibFile@@UAE_NXZ */
  if (*(int *)(this + 0x10) == 0) {
    bVar3 = false;
  }
  else {
    FUN_013e3470(*(undefined4 **)(this + 0x10));
    iVar1 = FUN_012a33f0(*(int *)(this + 0x10) + 4);
    piVar2 = (int *)GFxSprite__unknown_014b3780(*(undefined4 **)(this + 0x10));
    (**(code **)(*piVar2 + 0x48))();
    if (*(void **)(this + 0x10) != (void *)0x0) {
      FUN_013e34d0(*(void **)(this + 0x10),1);
    }
    *(undefined4 *)(this + 0x10) = 0;
    bVar3 = iVar1 == 0;
  }
  return bVar3;
}




/* public: virtual bool __thiscall GZLibFile::ChangeSize(int) */

bool __thiscall GZLibFile::ChangeSize(GZLibFile *this,int param_1)

{
                    /* 0xfe6980  54  ?ChangeSize@GZLibFile@@UAE_NH@Z */
  return false;
}




/* === Additional global functions for GZLibFile (1 functions) === */

GZLibFile * __thiscall GZLibFile__vfunc_0(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    GZLibFile::~GZLibFile(this);
    if ((param_1 & 1) != 0) {
      Gdiplus::GdiplusBase::operator_delete(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_(this,0x14,*(int *)((int)this + -4),GZLibFile::~GZLibFile);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free((int)this + -4);
    }
    this = (void *)((int)this + -4);
  }
  return this;
}




/* ========== Gdiplus.c ========== */

/*
 * SGW.exe - Gdiplus (1 functions)
 */


/* Library Function - Single Match
    public: static void __cdecl Gdiplus::GdiplusBase::operator delete(void *)
   
   Library: Visual Studio 2010 Debug */

void __cdecl Gdiplus::GdiplusBase::operator_delete(void *param_1)

{
  FUN_00455b50(param_1);
  return;
}





/* ========== UGFxEditor.c ========== */

/*
 * SGW.exe - UGFxEditor (1 functions)
 */

/* [VTable] UGFxEditor virtual function #2
   VTable: vtable_UGFxEditor at 0195ff94 */

int * __thiscall UGFxEditor__vfunc_2(void *this,byte param_1)

{
  FUN_00aedb90(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UGFxGarbageCollector.c ========== */

/*
 * SGW.exe - UGFxGarbageCollector (3 functions)
 */

/* [VTable] UGFxGarbageCollector virtual function #67
   VTable: vtable_UGFxGarbageCollector at 01961024 */

void UGFxGarbageCollector__vfunc_67(void)

{
  undefined **ppuVar1;
  
  ppuVar1 = (undefined **)GDebugAllocator__unknown_00455ae0();
  if (ppuVar1 != &PTR_vftable_01debb10) {
    FUN_00455ab0((undefined *)&PTR_vftable_01debb10);
  }
  return;
}


/* [VTable] UGFxGarbageCollector virtual function #11
   VTable: vtable_UGFxGarbageCollector at 01961024 */

void __fastcall UGFxGarbageCollector__vfunc_11(int param_1)

{
  undefined *this;
  int iVar1;
  
  iVar1 = 0;
  this = FlashExternalWindowModule__unknown_00569230();
  FUN_00568e80(this,iVar1);
  UTestIpDrv__vfunc_11(param_1);
  return;
}


/* [VTable] UGFxGarbageCollector virtual function #2
   VTable: vtable_UGFxGarbageCollector at 01961024 */

int * __thiscall UGFxGarbageCollector__vfunc_2(void *this,byte param_1)

{
  FUN_00af1d20(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UGFxMovieInfo.c ========== */

/*
 * SGW.exe - UGFxMovieInfo (2 functions)
 */

/* [VTable] UGFxMovieInfo virtual function #2
   VTable: vtable_UGFxMovieInfo at 01960e4c */

int * __thiscall UGFxMovieInfo__vfunc_2(void *this,byte param_1)

{
  FUN_00af1460(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UGFxMovieInfo virtual function #1
   VTable: vtable_UGFxMovieInfo at 01960e4c */

bool __fastcall UGFxMovieInfo__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ef0a30 == (undefined4 *)0x0) {
    DAT_01ef0a30 = FUN_00af1610();
    FUN_00af1190();
  }
  return puVar1 != DAT_01ef0a30;
}




/* ========== UGFxMovieThumbnailRenderer.c ========== */

/*
 * SGW.exe - UGFxMovieThumbnailRenderer (3 functions)
 */

/* [VTable] UGFxMovieThumbnailRenderer virtual function #1
   VTable: vtable_UGFxMovieThumbnailRenderer at 01960804 */

bool __fastcall UGFxMovieThumbnailRenderer__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ef2fd4 == (undefined4 *)0x0) {
    DAT_01ef2fd4 = FUN_00eef520();
    FUN_00eedc40();
  }
  return puVar1 != DAT_01ef2fd4;
}


/* [VTable] UGFxMovieThumbnailRenderer virtual function #67
   VTable: vtable_UGFxMovieThumbnailRenderer at 01960804 */

bool UGFxMovieThumbnailRenderer__vfunc_67(int param_1)

{
  return param_1 != 0;
}


/* [VTable] UGFxMovieThumbnailRenderer virtual function #2
   VTable: vtable_UGFxMovieThumbnailRenderer at 01960804 */

int * __thiscall UGFxMovieThumbnailRenderer__vfunc_2(void *this,byte param_1)

{
  FUN_00aeea50(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UGFxRawData.c ========== */

/*
 * SGW.exe - UGFxRawData (1 functions)
 */

/* [VTable] UGFxRawData virtual function #2
   VTable: vtable_UGFxRawData at 01960d3c */

int * __thiscall UGFxRawData__vfunc_2(void *this,byte param_1)

{
  FUN_00af12e0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UGFxUpdatableTexture.c ========== */

/*
 * SGW.exe - UGFxUpdatableTexture (2 functions)
 */

/* [VTable] UGFxUpdatableTexture virtual function #70
   VTable: vtable_UGFxUpdatableTexture at 0196226c */

void __fastcall UGFxUpdatableTexture__vfunc_70(int param_1)

{
  void *this;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0175e9fb;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  this = (void *)scalable_malloc(0x44,DAT_01e7f9a0 ^ (uint)&stack0xffffffe0);
  if (this == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 0;
  FUN_00afc2b0(this,*(undefined4 *)(param_1 + 200),*(undefined4 *)(param_1 + 0xcc),
               *(undefined4 *)(param_1 + 0xd8),(uint)*(byte *)(param_1 + 0xd0));
  ExceptionList = local_c;
  return;
}


/* [VTable] UGFxUpdatableTexture virtual function #2
   VTable: vtable_UGFxUpdatableTexture at 0196226c */

int * __thiscall UGFxUpdatableTexture__vfunc_2(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016db828;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_0075cf50(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== VGASEnvironment.c ========== */

/*
 * SGW.exe - VGASEnvironment (1 functions)
 */

undefined4 * __thiscall VGASEnvironment___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_01461fb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASArrayObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASArrayObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASArrayObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014c0240(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASAsBroadcaster.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASAsBroadcaster (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASAsBroadcaster___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_0149d5f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASBitmapData.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASBitmapData (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASBitmapData___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FID__unknown_014acdd0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASBooleanObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASBooleanObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASBooleanObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014d8fc0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASButtonObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASButtonObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASButtonObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014668e0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASColorObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASColorObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASColorObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014d80c0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASColorTransformObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASColorTransformObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASColorTransformObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014d0e10(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASDate.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASDate (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASDate___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014cdc30(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASExternalInterface.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASExternalInterface (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASExternalInterface___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014ca860(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASLoadVarsObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASLoadVarsObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASLoadVarsObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014c7c60(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASMatrixObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASMatrixObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASMatrixObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014d5590(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASMouse.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASMouse (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASMouse___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_0149f600(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASMovieClipLoader.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASMovieClipLoader (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASMovieClipLoader___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014c9390(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASMovieClipObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASMovieClipObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASMovieClipObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014624f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASNumberObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASNumberObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASNumberObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FID__unknown_014da060(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FID__unknown_0146a670(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASPointObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASPointObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASPointObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014d3090(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASRectangleObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASRectangleObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASRectangleObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014ab980(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASSoundObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASSoundObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASSoundObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FID__unknown_014d8810(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASStageObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASStageObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASStageObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014cf2b0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASStringObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASStringObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASStringObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FID__unknown_014dc100(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASTextFieldObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASTextFieldObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASTextFieldObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_013f9770(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASTextFormatObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASTextFormatObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASTextFormatObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014a7d40(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASTransformObject.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASTransformObject (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASTransformObject___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014d6dd0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASXml.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASXml (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASXml___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014c6670(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASXmlNode.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASXmlNode (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASXmlNode___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_014c58f0(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASEnvironment_VGASXmlSocket.c ========== */

/*
 * SGW.exe - VGASEnvironment_VGASXmlSocket (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGASEnvironment_VGASXmlSocket___GASPrototype__vfunc_0(void *this,uint param_1)

{
  FUN_013e1320(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGASFnCall.c ========== */

/*
 * SGW.exe - VGASFnCall (1 functions)
 */

undefined4 * __thiscall VGASFnCall___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_01428f60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxActionLogger.c ========== */

/*
 * SGW.exe - VGFxActionLogger (1 functions)
 */

undefined4 * __thiscall VGFxActionLogger___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_01428fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxCharacter.c ========== */

/*
 * SGW.exe - VGFxCharacter (1 functions)
 */

undefined4 * __thiscall VGFxCharacter___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_0147a4d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxLoadProcess.c ========== */

/*
 * SGW.exe - VGFxLoadProcess (1 functions)
 */

undefined4 * __thiscall VGFxLoadProcess___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_014070e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxLoaderImpl.c ========== */

/*
 * SGW.exe - VGFxLoaderImpl (1 functions)
 */

undefined4 * __thiscall VGFxLoaderImpl___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_013d9fb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxLog.c ========== */

/*
 * SGW.exe - VGFxLog (1 functions)
 */

undefined4 * __thiscall VGFxLog___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_013d9ec0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxSharedStateImpl.c ========== */

/*
 * SGW.exe - VGFxSharedStateImpl (1 functions)
 */

undefined4 * __thiscall VGFxSharedStateImpl___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_013d9f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxStream.c ========== */

/*
 * SGW.exe - VGFxStream (1 functions)
 */

undefined4 * __thiscall VGFxStream___GFxLogBase__vfunc_0(void *this,uint param_1)

{
  FUN_013e5320(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VGFxSwfPathData_VGFxConstShapeNoStylesDef.c ========== */

/*
 * SGW.exe - VGFxSwfPathData_VGFxConstShapeNoStylesDef (1 functions)
 */

GRefCountBaseImpl * __thiscall
VGFxSwfPathData_VGFxConstShapeNoStylesDef___GFxVirtualPathIterator__vfunc_0(void *this,uint param_1)

{
  ~_CancellationTokenCallback<>(this);
  if ((param_1 & 1) != 0) {
    Gdiplus::GdiplusBase::operator_delete(this);
  }
  return this;
}




/* ========== VGImage.c ========== */

/*
 * SGW.exe - VGImage (1 functions)
 */

/* Also in vtable: VGFxState___GRefCountBase (slot 0)
   Also in vtable: VGImage___GRefCountBase (slot 0) */

GRefCountBaseImpl * __thiscall VGFxState___GRefCountBase__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0168e1b8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== VGRenderer.c ========== */

/*
 * SGW.exe - VGRenderer (1 functions)
 */

/* Also in vtable: VGImageInfoBase___GRefCountBase (slot 0)
   Also in vtable: VGRenderer___GRefCountBase (slot 0)
   Also in vtable: VGFile___GRefCountBase (slot 0) */

GRefCountBaseImpl * __thiscall VGFile___GRefCountBase__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016da1f8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  GRefCountBaseImpl::~GRefCountBaseImpl(this);
  if ((param_1 & 1) != 0) {
    FUN_00455b50(this);
  }
  ExceptionList = local_c;
  return this;
}




/* === Standalone functions (7) === */

/* [VTable] GSemaphoreWaitableIncrement virtual function #5
   VTable: vtable_GSemaphoreWaitableIncrement at 017fbb68
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 7) */

undefined1 GSemaphoreWaitableIncrement__vfunc_5(void)

{
  return 1;
}


/* [VTable] FGFxRenderer_FGFxRenderStyle virtual function #2
   VTable: vtable_FGFxRenderer_FGFxRenderStyle at 0196135c
   Also in vtable: FGFxRenderer_FGFxLineStyle (slot 2) */

void __thiscall FGFxRenderer_FGFxRenderStyle__vfunc_2(void *this,undefined4 *param_1,int *param_2)

{
  if (*(int *)((int)this + 4) == 0) {
    FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x5bc);
  }
  if (*param_2 == 1) {
    param_1[1] = 0x1000;
    param_1[2] = 0x80000;
    *param_1 = 1;
    return;
  }
  *param_1 = 1;
  return;
}


/* Also in vtable: FGFxRenderer_FGFxRenderStyle (slot 0)
   Also in vtable: FGFxRenderer_FGFxLineStyle (slot 0) */

void __fastcall FGFxRenderer_FGFxLineStyle__vfunc_0(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 8) = 0xff0000;
  return;
}


/* [VTable] FGFxRenderer_FGFxRenderStyle virtual function #1
   VTable: vtable_FGFxRenderer_FGFxRenderStyle at 0196135c
   Also in vtable: FGFxRenderer_FGFxLineStyle (slot 1) */

void __thiscall
FGFxRenderer_FGFxRenderStyle__vfunc_1(void *this,undefined4 param_1,undefined4 *param_2,int param_3)

{
  int *piVar1;
  
  if (*(int *)((int)this + 4) == 0) {
    FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x599);
    if (*(int *)((int)this + 4) == 0) {
      FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x5a1);
    }
  }
  FUN_00af3a20(param_1,*(uint *)((int)this + 8),*(int *)(param_3 + 4),(int *)*param_2);
  if (*(int *)((int)this + 4) == 0) {
    FUN_00486000("GFx_SM_Disabled != StyleMode",".\\Src\\GFxUIRenderer.cpp",0x5ae);
  }
  piVar1 = (int *)(**(code **)(*(int *)*param_2 + 0x14))();
  piVar1 = FUN_00540270(piVar1);
  (**(code **)(*(int *)*param_2 + 0xc))(*piVar1,(int)this + 0xc);
  return;
}


/* [String discovery] Debug string: "---- GFxSwfEvent::Read -- Event_KeyPress found, key code is
   %d\n"
   String at: 01af2290 */

void __thiscall GFxSwfEvent_Read(void *this,void *param_1,undefined4 param_2)

{
  bool bVar1;
  undefined4 uVar2;
  int iVar3;
  GRefCountBaseImpl *pGVar4;
  void *this_00;
  void *pvVar5;
  GRefCountBaseImpl *local_38;
  undefined4 local_28;
  undefined4 local_24;
  undefined4 local_20;
  uint local_1c;
  uint local_18;
  int local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_01785ccb;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  GFxSprite__unknown_013e0760(&local_28,param_2,0,0,0);
  *(undefined4 *)this = local_28;
  *(undefined4 *)((int)this + 4) = local_24;
  *(undefined4 *)((int)this + 8) = local_20;
  local_14 = GFxSprite__unknown_013d5d70(param_1);
  if ((*(uint *)this & 0x20000) != 0) {
    uVar2 = GFxSprite__unknown_013db050(param_1);
    *(ushort *)((int)this + 8) = (ushort)uVar2 & 0xff;
    local_14 = local_14 + -1;
    GFxSprite__unknown_013da230
              ((int)param_1,"---- GFxSwfEvent::Read -- Event_KeyPress found, key code is %d\n");
  }
  bVar1 = FUN_013e5030((int)param_1);
  if (bVar1) {
    iVar3 = FUN_01427e10(this);
    FUN_014c16f0(iVar3);
    FUN_0142b110((int)param_1,"---- actions for event 0x%X (%s)\n");
  }
  pGVar4 = (GRefCountBaseImpl *)GFxSprite__unknown_00453600(0x20);
  local_8 = 0;
  if (pGVar4 == (GRefCountBaseImpl *)0x0) {
    local_38 = (GRefCountBaseImpl *)0x0;
  }
  else {
    local_38 = FUN_0141c830(pGVar4);
  }
  local_8 = 0xffffffff;
  GFxSprite__unknown_014dedc0((void *)((int)this + 0xc),(int)local_38);
  pvVar5 = param_1;
  this_00 = (void *)GFxSprite__unknown_014b3780((undefined4 *)((int)this + 0xc));
  GSemaphoreWaitableIncrement__unknown_0141cb20(this_00,pvVar5);
  iVar3 = GFxSprite__unknown_014b3780((undefined4 *)((int)this + 0xc));
  iVar3 = GFxSwfEvent__unknown_0141d010(iVar3);
  if (iVar3 != local_14) {
    iVar3 = GFxSprite__unknown_014b3780((undefined4 *)((int)this + 0xc));
    GFxSwfEvent__unknown_0141d010(iVar3);
    FUN_013deba0((int)param_1,"Error: GFxSwfEvent::Read - EventLength = %d, but read %d\n");
    iVar3 = GFxSprite__unknown_014b3780((undefined4 *)((int)this + 0xc));
    iVar3 = GFxSwfEvent__unknown_0141d010(iVar3);
    local_1c = local_14 - iVar3;
    for (local_18 = 0; local_18 < local_1c; local_18 = local_18 + 1) {
      FUN_0143fe00(param_1);
    }
  }
  ExceptionList = local_10;
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [String discovery] Debug string: "GFxTextDocView::Format() - missing glyph %d. Make sure that SWF
   file includes character shapes for \"%s\" font.\n"
   String at: 01af4be8 */

void __thiscall GFxTextDocView_Format(void *this,int param_1,int param_2)

{
  ushort uVar1;
  bool bVar2;
  char cVar3;
  undefined2 uVar4;
  undefined2 uVar5;
  ushort uVar6;
  int iVar7;
  CPropertySection *this_00;
  uint *puVar8;
  uint uVar9;
  uint uVar10;
  undefined4 *puVar11;
  Count *pCVar12;
  void *this_01;
  undefined2 extraout_var;
  undefined2 extraout_var_00;
  undefined2 extraout_var_01;
  float *pfVar13;
  int iVar14;
  undefined4 extraout_ECX;
  undefined4 extraout_ECX_00;
  undefined4 extraout_ECX_01;
  undefined4 uVar15;
  undefined4 extraout_ECX_02;
  undefined4 extraout_ECX_03;
  undefined4 extraout_ECX_04;
  undefined4 extraout_EDX;
  undefined4 extraout_EDX_00;
  undefined4 uVar16;
  undefined4 extraout_EDX_01;
  undefined4 extraout_EDX_02;
  undefined4 extraout_EDX_03;
  float10 fVar17;
  ulonglong uVar18;
  undefined1 *puVar19;
  float fVar20;
  GPoint<float> *pGVar21;
  char local_320;
  undefined1 local_314 [8];
  undefined4 local_30c [43];
  undefined1 *local_260;
  undefined1 local_25c [20];
  int local_248;
  int local_244;
  GPoint<float> local_240 [4];
  float local_23c;
  int local_238 [5];
  char local_221;
  uint local_220;
  float local_21c;
  int local_218;
  uint local_214;
  int local_210;
  void *local_20c;
  int local_208;
  uint local_204;
  char local_1fd;
  int local_1fc;
  int local_1f8;
  void *local_1f4;
  uint local_1f0;
  float local_1ec;
  int local_1e8;
  undefined4 *local_1e4;
  float local_1e0;
  float local_1dc;
  int local_1d8;
  float local_1d4;
  undefined4 local_1d0;
  int local_1cc;
  void *local_1c8;
  undefined4 *local_1c4;
  undefined4 local_1c0 [33];
  undefined4 local_13c [12];
  int local_10c;
  uint local_108;
  int local_104;
  void *local_100;
  uint local_fc;
  uint *local_f8;
  uint local_f4;
  int local_f0;
  IUnknown local_ec [4];
  uint local_dc;
  void *local_d8;
  int local_d4;
  uint local_d0;
  int local_cc;
  float local_c8;
  uint local_c4;
  int local_c0;
  undefined4 local_bc;
  ulong local_b8;
  int local_b4;
  int local_a4;
  int local_a0;
  int local_9c;
  float local_98;
  float local_94;
  undefined1 local_8c [44];
  int local_60;
  int local_5c;
  int local_58;
  int local_54 [5];
  int local_40;
  int local_3c;
  float local_38;
  char local_34;
  undefined1 local_33;
  int local_24;
  int local_20;
  undefined4 local_18;
  char local_11;
  void *local_10;
  undefined1 *puStack_c;
  int local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_01789c2a;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_01487a00(&local_24,*(undefined4 *)(param_2 + 0xd0));
  local_8 = 0;
  local_100 = (void *)GFxTextDocView__unknown_01406db0(param_1);
  FUN_01487cf0(&local_d8,this,param_1,(int *)(param_2 + 0x4e0));
  local_8._0_1_ = 1;
  FUN_01487b10(local_1c0);
  local_8 = CONCAT31(local_8._1_3_,2);
  fVar17 = (float10)FUN_013eef70((int)this);
  local_38 = (float)fVar17;
  bVar2 = FUN_013e8d70((int)this);
  if (bVar2) {
    iVar7 = GFxSprite__unknown_013e7210((int)this);
    cVar3 = FUN_01487ac0(iVar7);
    if (cVar3 != '\0') {
      iVar7 = GFxSprite__unknown_013e7210((int)this);
      iVar7 = GFxTextDocView__unknown_01435d00(iVar7);
      GFxTextDocView__unknown_013e1050(&local_bc,iVar7);
      this_00 = (CPropertySection *)GFxSprite__unknown_014b3780(&local_bc);
      local_b8 = CPropertySection::GetCount(this_00);
      iVar7 = GFxSprite__unknown_014b3780(&local_bc);
      local_b4 = GFxTextDocView__unknown_013f6160(iVar7);
    }
  }
  local_f0 = GFxTextDocView__unknown_0147fb20(param_1);
  local_f0 = local_f0 + local_b4;
  cVar3 = FUN_0147db90((int)local_100);
  if (cVar3 != '\0') {
    local_f0 = local_f0 + 1;
  }
  local_fc = FUN_014a0280(local_f0,local_f0 << 1,1);
  local_f8 = (uint *)0x0;
  if (local_fc < 0x400) {
    local_f8 = (uint *)(param_2 + 0xd4);
    FUN_01487940(local_f8,local_fc);
  }
  else if (*(int *)(param_2 + 0x4d4) == 0) {
    puVar8 = FUN_0149f730(local_fc + 100,1);
    *(uint **)(param_2 + 0x4d4) = puVar8;
    local_f8 = *(uint **)(param_2 + 0x4d4);
  }
  else {
    uVar9 = FUN_01487970(*(uint **)(param_2 + 0x4d4));
    if (uVar9 <= local_fc) {
      FUN_0149f830(*(uint **)(param_2 + 0x4d4));
      puVar8 = FUN_0149f730(local_fc + 100,1);
      *(uint **)(param_2 + 0x4d4) = puVar8;
    }
    local_f8 = *(uint **)(param_2 + 0x4d4);
  }
  FUN_01487870(local_f8);
  FUN_01487920(local_f8);
  FUN_01487990(local_f8,local_f0);
  uVar9 = GFxTextDocView__unknown_01451da0(param_1);
  uVar9 = FUN_0148cd00(this,uVar9);
  GFxTextDocView__unknown_01485400(local_f8,uVar9);
  uVar9 = GFxTextDocView__unknown_0149fc80(local_f8);
  uVar10 = FUN_013ff4c0(local_f8);
  iVar7 = GFxTextDocView__unknown_013ff520(local_f8);
  puVar11 = FUN_01430fa0(local_25c,iVar7,uVar10,uVar9);
  FUN_014877a0(local_54,puVar11);
  *(undefined4 *)(param_2 + 0x504) = 0;
  *(undefined4 *)(param_2 + 0x508) = 0;
  *(undefined4 *)(param_2 + 0x50c) = 0;
  IUnknown::IUnknown(local_ec);
  cVar3 = FUN_0147db90((int)local_100);
  if (cVar3 == '\0') {
    iVar7 = FUN_0147dc40((int)local_100);
    local_60 = GFxTextDocView__unknown_013fa870(iVar7);
  }
  else {
    local_1c8 = (void *)FUN_01431050(local_54);
    local_1c4 = (undefined4 *)GFxTextDocView__unknown_01488120((int)&local_d8);
    local_20 = GFxSprite__unknown_014b3780(local_1c4);
    if (local_20 != 0) {
      pCVar12 = (Count *)GFxTextDocView__unknown_01485590(this,&local_24);
      FUN_014907f0(&local_1d8,pCVar12);
      local_8._0_1_ = 3;
      uVar4 = 0x25cf;
      iVar7 = GFxSprite__unknown_014b3780(&local_1d8);
      this_01 = (void *)GFxTextDocView__unknown_01406db0(iVar7);
      local_1d0 = GFxTextDocView__unknown_01487500(this_01,uVar4);
      FUN_01430ec0(local_1c8,(short)local_1d0);
      iVar7 = GFxTextDocView__unknown_013fa870(0xf);
      GFxTextDocView__unknown_01430f10(local_1c8,iVar7);
      fVar17 = FUN_01488720((int)this,(int)&local_24,param_2);
      local_1d4 = (float)((fVar17 + fVar17) / (float10)_DAT_0183db60);
      FUN_014875d0(local_1c8,local_1d4);
      GFxTextDocView__unknown_014875a0(local_1c8,0);
      local_1cc = FUN_014471f0(local_20);
      pCVar12 = (Count *)GFxSprite__unknown_014b3780(&local_1d8);
      FUN_014310a0(local_54,pCVar12);
      local_260 = &stack0xfffffcb0;
      FUN_00aef690(&stack0xfffffcb0,local_1cc);
      FUN_01431140(local_54);
      FUN_01430fe0(local_54);
      GFxTextDocView__unknown_013f9470(&local_d4,&local_1d8);
      local_c0 = local_1cc;
      local_8 = CONCAT31(local_8._1_3_,2);
      GFxTextDocView__unknown_013f9340(&local_1d8);
    }
    local_60 = GFxTextDocView__unknown_013fa870(0x14);
    local_a4 = GFxTextDocView__unknown_013fa870(0xf);
  }
  uVar4 = FUN_0147dcc0((int)local_100);
  uVar5 = FUN_0147dc00((int)local_100);
  local_5c = GFxTextDocView__unknown_013fa870
                       (CONCAT22(extraout_var,uVar4) + CONCAT22(extraout_var_00,uVar5));
  uVar4 = FUN_0147dd00((int)local_100);
  local_58 = GFxTextDocView__unknown_013fa870(CONCAT22(extraout_var_01,uVar4));
  local_dc = 0;
  local_f4 = 0;
  local_10c = FUN_0147e200(local_100,&local_dc);
  local_108 = 1;
  FUN_01441890(&local_104);
  local_8 = CONCAT31(local_8._1_3_,4);
  local_11 = '\0';
  do {
    cVar3 = FUN_01487e90((int)&local_d8);
    if (cVar3 != '\0') {
      if (local_f8 != (uint *)0x0) {
        GFxTextDocView__unknown_01485780
                  ((int)this,param_2,local_f8,param_2,(int)local_100,(int *)&local_d8);
      }
      local_8._0_1_ = 2;
      GFxSprite__unknown_013d9ff0(&local_104);
      local_8._0_1_ = 1;
      GFxTextDocView__unknown_014887f0((int)local_1c0);
      local_8 = (uint)local_8._1_3_ << 8;
      GFxTextDocView__unknown_014887f0((int)&local_d8);
      local_8 = 0xffffffff;
      FUN_014887d0((int)&local_24);
      ExceptionList = local_10;
      return;
    }
    bVar2 = GFxTextDocView__unknown_013ec060((int)this);
    if (((bVar2) && (iVar7 = GFxSprite__unknown_014b3780(&local_104), iVar7 == 0)) &&
       ((local_d0 == 0x2d || (local_11 != '\0')))) {
      GFxTextDocView__unknown_01488870(local_1c0,&local_d8);
    }
    local_1e4 = (undefined4 *)GFxTextDocView__unknown_01488120((int)&local_d8);
    local_108 = 1;
    GFxTextDocView__unknown_013e1050(&local_104,0);
    if (*(int *)((int)this + 0x20) != 0) {
      local_210 = FUN_014804c0(local_8c,&local_218);
      iVar7 = FUN_014843a0(*(void **)((int)this + 0x20),local_210,local_218,&local_214);
      GFxTextDocView__unknown_013e1050(&local_104,iVar7);
      iVar7 = GFxSprite__unknown_014b3780(&local_104);
      if (iVar7 != 0) {
        local_108 = local_214;
      }
    }
    iVar7 = GFxSprite__unknown_014b3780(&local_104);
    if ((iVar7 == 0) && (iVar7 = GFxSprite__unknown_014b3780(local_1e4), iVar7 != 0)) {
      iVar7 = GFxSprite__unknown_014b3780(local_1e4);
      bVar2 = FUN_0147d020(iVar7);
      if ((bVar2) && (*(short *)(local_1e4 + 2) != 0)) {
        iVar7 = GFxSprite__unknown_014b3780(local_1e4);
        iVar7 = FUN_0147cea0(iVar7);
        GFxTextDocView__unknown_013e1050(&local_104,iVar7);
      }
    }
    local_20c = (void *)0x0;
    FUN_01441890(&local_208);
    local_8 = CONCAT31(local_8._1_3_,5);
    local_1e0 = 1.0;
    local_1ec = DAT_01814190;
    iVar7 = GFxSprite__unknown_014b3780(&local_104);
    if (iVar7 == 0) {
      local_1fd = *(short *)(local_1e4 + 2) == 0xa0;
      if ((*(short *)(local_1e4 + 2) == 0) ||
         ((!(bool)local_1fd && (iVar7 = FUN_014874e0(*(wint_t *)(local_1e4 + 2)), iVar7 != 0)))) {
        local_320 = '\x01';
      }
      else {
        local_320 = '\0';
      }
      local_11 = local_320;
      iVar7 = GFxSprite__unknown_014b3780(&local_18);
      if ((iVar7 != 0) && (iVar7 = GFxSprite__unknown_014b3780(local_1e4), local_20 == iVar7)) {
        GFxTextDocView__unknown_013f9470(&local_208,&local_18);
LAB_01486b31:
        iVar7 = GFxSprite__unknown_014b3780(&local_208);
        local_20c = (void *)GFxTextDocView__unknown_01406db0(iVar7);
        fVar17 = FUN_01488720((int)this,(int)&local_24,param_2);
        local_1ec = (float)fVar17;
        fVar17 = FUN_013fa710(local_1ec);
        local_1e0 = (float)(fVar17 / (float10)_DAT_0193f318);
        uVar1 = *(ushort *)(local_1e4 + 2);
        uVar6 = FUN_01487aa0((int)this);
        if ((uVar1 == (uVar6 & 0xff)) || (*(short *)(local_1e4 + 2) == 0)) {
          local_1e8 = GFxTextDocView__unknown_01487500(local_20c,0x20);
          fVar17 = (float10)FUN_01487530(local_20c,local_1e8);
          fVar17 = (fVar17 / (float10)DAT_01866fe0) * (float10)local_1e0;
          uVar15 = extraout_ECX;
          uVar16 = extraout_EDX;
        }
        else {
          local_1e8 = GFxTextDocView__unknown_01487500(local_20c,*(undefined2 *)(local_1e4 + 2));
          if (((local_1e8 == -1) && (*(int *)(param_2 + 0x500) != 0)) && (DAT_01f11b28 < 10)) {
            DAT_01f11b28 = DAT_01f11b28 + 1;
            FUN_013e8810((int)local_20c);
            FID__unknown_013cd230
                      (*(int *)(param_2 + 0x500) + 0x14,
                       "GFxTextDocView::Format() - missing glyph %d. Make sure that SWF file includes character shapes for \"%s\" font.\n"
                      );
          }
          fVar17 = (float10)FUN_01487530(local_20c,local_1e8);
          fVar17 = fVar17 * (float10)local_1e0;
          uVar15 = extraout_ECX_00;
          uVar16 = extraout_EDX_00;
        }
        local_1dc = (float)fVar17;
        if (local_d8 == (void *)0x0) {
          uVar18 = GFxTextDocView__unknown_01495ea0(uVar15,uVar16);
          local_1f8 = (int)uVar18;
        }
        else {
          local_21c = 0.0;
          if (local_34 != '\0') {
            fVar17 = FUN_014883c0((int *)&local_d8,*(ushort *)(local_1e4 + 2));
            local_21c = (float)(fVar17 * (float10)local_1e0);
            uVar15 = extraout_ECX_01;
            uVar16 = extraout_EDX_01;
          }
          uVar18 = GFxTextDocView__unknown_01495ea0(uVar15,uVar16);
          local_1f8 = (int)uVar18;
        }
        fVar20 = local_1dc;
        iVar7 = FUN_013febc0(local_20c,local_1e8,local_ec);
        GFxTextDocView__unknown_00aef3e0
                  (*(float *)(iVar7 + 8) * local_1e0 + (float)_DAT_0193f358,fVar20);
        uVar18 = GFxTextDocView__unknown_01495ea0(extraout_ECX_02,extraout_EDX_02);
        local_1f0 = (uint)uVar18;
        local_204 = ~-(uint)(local_11 != '\0') & local_1f0;
        goto LAB_01486deb;
      }
      local_20 = GFxSprite__unknown_014b3780(local_1e4);
      pCVar12 = (Count *)GFxTextDocView__unknown_01485590(this,&local_24);
      GFxTextDocView__unknown_01490850(&local_208,pCVar12);
      iVar7 = GFxSprite__unknown_014b3780(&local_208);
      if (iVar7 != 0) goto LAB_01486b31;
      local_8 = CONCAT31(local_8._1_3_,4);
      GFxTextDocView__unknown_013f9340(&local_208);
    }
    else {
      local_20 = GFxSprite__unknown_014b3780(local_1e4);
      iVar7 = GFxSprite__unknown_014b3780(&local_104);
      FUN_01487560(iVar7);
      uVar18 = GFxTextDocView__unknown_01495ea0(extraout_ECX_03,extraout_EDX_03);
      local_1f0 = (uint)uVar18;
      local_1dc = (float)(int)local_1f0 + (float)_DAT_01885248;
      uVar18 = GFxTextDocView__unknown_01495ea0(extraout_ECX_04,(int)(uVar18 >> 0x20));
      local_1f8 = (int)uVar18;
      local_204 = local_1f0;
      local_1e8 = -1;
      local_11 = '\0';
      local_1fd = '\0';
LAB_01486deb:
      local_1fc = local_a4 + local_1f8;
      bVar2 = GFxTextDocView__unknown_013ec060((int)this);
      if ((bVar2) && (local_11 == '\0')) {
        iVar7 = local_1fc + local_204 + local_60 + local_5c;
        pfVar13 = (float *)GFxTextDocView__unknown_01448af0((int)this);
        fVar17 = GFxTextDocView__unknown_013d9ef0(pfVar13);
        if (fVar17 - (float10)local_58 < (float10)iVar7) {
          local_1fc = 0;
          local_221 = '\0';
          cVar3 = FUN_01487e90((int)local_1c0);
          if (cVar3 == '\0') {
            FUN_01487780(local_238,local_13c);
            FUN_0149f9a0(local_54,local_238);
            GFxTextDocView__unknown_01488870(&local_d8,local_1c0);
            local_d0 = 0;
            local_11 = '\0';
            puVar11 = FUN_01487b10(local_30c);
            local_8._0_1_ = 6;
            GFxTextDocView__unknown_01488870(local_1c0,puVar11);
            local_8 = CONCAT31(local_8._1_3_,5);
            GFxTextDocView__unknown_014887f0((int)local_30c);
            local_221 = '\x01';
            local_108 = 0;
          }
          iVar7 = GFxTextDocView__unknown_01451da0(param_1);
          iVar14 = GFxTextDocView__unknown_01488120((int)&local_d8);
          local_220 = iVar7 + *(int *)(iVar14 + 4);
          if (local_d8 != (void *)0x0) {
            FUN_01487760((int)local_d8);
          }
          GFxTextDocView__unknown_01485780
                    ((int)this,local_f8,local_f8,param_2,(int)local_100,(int *)&local_d8);
          FUN_01488440(&local_d8);
          local_f4 = 0;
          GFxTextDocView__unknown_01485400(local_f8,local_220);
          if (local_221 != '\0') {
            local_a4 = 0;
            local_8 = CONCAT31(local_8._1_3_,4);
            GFxTextDocView__unknown_013f9340(&local_208);
            goto LAB_014868a9;
          }
        }
      }
      if (local_d8 != (void *)0x0) {
        GFxTextDocView__unknown_01430f10(local_d8,local_1f8);
      }
      local_1f4 = (void *)FUN_01431050(local_54);
      FUN_01430ec0(local_1f4,(short)local_1e8);
      uVar9 = GFxTextDocView__unknown_01488050((int)&local_d8);
      if ((uVar9 & 0xff) != 0) {
        FUN_01487680((int)local_1f4);
      }
      iVar7 = GFxSprite__unknown_014b3780(&local_104);
      if (iVar7 == 0) {
        local_244 = FUN_014471f0(local_20);
        FUN_014875d0(local_1f4,local_1ec);
        bVar2 = FUN_01471d30(&local_208,&local_d4);
        if (bVar2) {
          pCVar12 = (Count *)GFxSprite__unknown_014b3780(&local_208);
          FUN_014310a0(local_54,pCVar12);
        }
        if (local_244 != local_c0) {
          FUN_00aef690(&stack0xfffffcb0,local_244);
          FUN_01431140(local_54);
        }
        uVar1 = *(ushort *)(local_1e4 + 2);
        uVar6 = FUN_01487aa0((int)this);
        if ((uVar1 == (uVar6 & 0xff)) || (*(short *)(local_1e4 + 2) == 0)) {
          if (*(short *)(local_1e4 + 2) == 0) {
            GFxTextDocView__unknown_014875a0(local_1f4,0);
          }
          else {
            GFxTextDocView__unknown_014875a0(local_1f4,1);
          }
          FUN_014876e0((int)local_1f4);
          FUN_01487700((int)local_1f4);
          local_1f0 = 0;
          local_33 = 1;
          if (local_1e4[1] == 0) {
            GFxTextDocView__unknown_014884d0(&local_d8,(int)local_20c,local_1e0);
          }
        }
        else {
          if ((local_11 != '\0') || (local_1fd != '\0')) {
            FUN_014876e0((int)local_1f4);
          }
          if (local_11 == '\0') {
            local_3c = 0;
            local_a0 = local_1fc + local_204;
          }
          else {
            local_40 = local_40 + 1;
            local_3c = local_3c + 1;
          }
          GFxTextDocView__unknown_014875a0(local_1f4,1);
          GFxTextDocView__unknown_014884d0(&local_d8,(int)local_20c,local_1e0);
        }
        bVar2 = FUN_0147cc70(local_20);
        if (bVar2) {
          FUN_014876a0((int)local_1f4);
        }
        else {
          FUN_014876c0((int)local_1f4);
        }
        cVar3 = GFxTextDocView__unknown_013efc90(local_20);
        if (cVar3 == '\0') {
          FUN_01487740((int)local_1f4);
        }
        else {
          FUN_01487720((int)local_1f4);
        }
        GFxTextDocView__unknown_013f9470(&local_d4,&local_208);
        local_c0 = local_244;
        local_34 = FUN_0147cc90(local_20);
      }
      else {
        iVar7 = GFxSprite__unknown_014b3780(&local_104);
        FUN_014877f0(local_54,iVar7);
        GFxTextDocView__unknown_014875a0(local_1f4,local_108);
        local_34 = '\0';
        GFxSprite__unknown_013f9bd0(local_240,0,0);
        pGVar21 = local_240;
        puVar19 = local_314;
        iVar7 = GFxSprite__unknown_014b3780(&local_104);
        puVar11 = FUN_013feab0((void *)(iVar7 + 0x28),puVar19,pGVar21);
        FUN_01400e40(local_240,puVar11);
        fVar17 = GFxTextDocView__unknown_00aef3e0(local_98,-local_23c);
        local_98 = (float)fVar17;
        iVar7 = GFxSprite__unknown_014b3780(&local_104);
        fVar17 = FUN_01487580(iVar7);
        fVar17 = GFxTextDocView__unknown_00aef3e0(local_94,(float)(fVar17 - -(float10)local_23c));
        local_94 = (float)fVar17;
        local_3c = 0;
        local_a0 = local_1fc + local_204;
      }
      if ((local_f4 < local_dc) && (*(short *)(local_1e4 + 2) == 9)) {
        local_248 = GFxTextDocView__unknown_013fa870(*(int *)(local_10c + local_f4 * 4));
        if (local_1fc < local_248) {
          local_1dc = (float)(local_248 - local_1fc);
        }
        local_f4 = local_f4 + 1;
      }
      iVar7 = FUN_013ff0b0((int)local_1f4);
      local_9c = iVar7 + local_9c;
      local_d8 = local_1f4;
      local_d0 = (uint)*(ushort *)(local_1e4 + 2);
      local_a4 = local_1fc;
      local_cc = local_1e8;
      local_c8 = local_1dc;
      bVar2 = FUN_0147d080(local_20);
      if (bVar2) {
        fVar17 = (float10)FUN_0147d590(local_20);
        fVar17 = FUN_013fa710((float)fVar17);
        local_c8 = (float)(fVar17 + (float10)local_c8);
      }
      local_c4 = local_1f0;
      FUN_01430fe0(local_54);
      local_8 = CONCAT31(local_8._1_3_,4);
      GFxTextDocView__unknown_013f9340(&local_208);
    }
LAB_014868a9:
    GFxTextDocView__unknown_01487eb0(&local_d8,local_108);
  } while( true );
}


/* [VTable] GSemaphoreWaitableIncrement virtual function #4
   VTable: vtable_GSemaphoreWaitableIncrement at 017fbb68
   Also in vtable: GMutex_AreadyLockedAcquireInterface (slot 6) */

void GSemaphoreWaitableIncrement__vfunc_4(void)

{
  return;
}


