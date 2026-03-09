/*
 * SGW.exe Decompilation - 13_other_game
 * Classes: 870
 * Stargate Worlds Client
 */


/* ========== A0x45c6b2f3_FDecalMaterial.c ========== */

/*
 * SGW.exe - A0x45c6b2f3_FDecalMaterial (1 functions)
 */

/* [VTable] A0x45c6b2f3_FDecalMaterial virtual function #5
   VTable: vtable__A0x45c6b2f3_FDecalMaterial at 018c789c */

undefined4 __thiscall
A0x45c6b2f3_FDecalMaterial__vfunc_5(void *this,undefined4 param_1,int *param_2)

{
  int iVar1;
  undefined4 uVar2;
  undefined4 uVar3;
  undefined4 uVar4;
  byte local_4c [8];
  undefined8 *puStack_44;
  undefined4 uStack_40;
  byte abStack_3c [8];
  undefined4 uStack_34;
  undefined4 uStack_1c;
  byte abStack_18 [24];
  
  uStack_34 = 0;
  abStack_3c[4] = 1;
  abStack_3c[5] = 0;
  abStack_3c[6] = 0;
  abStack_3c[7] = 0;
  abStack_3c[0] = 1;
  abStack_3c[1] = 0;
  abStack_3c[2] = 0;
  abStack_3c[3] = 0;
  uStack_40 = 1;
  puStack_44 = &DAT_01db0740;
  local_4c[0] = 0x4d;
  local_4c[1] = 3;
  local_4c[2] = 0;
  local_4c[3] = 0;
  local_4c[4] = 0;
  local_4c[5] = 0;
  local_4c[6] = 0;
  local_4c[7] = 0;
  uVar2 = (**(code **)(*param_2 + 0x10))();
  uVar2 = (**(code **)(*param_2 + 0xc4))(uVar2);
  switch(uStack_1c) {
  case 0:
    iVar1 = *param_2;
    uVar4 = 4;
    abStack_18[0] = 0;
    abStack_18[1] = 0;
    abStack_18[2] = 0;
    abStack_18[3] = 0xff;
    uVar3 = A0x45c6b2f3_FDecalMaterial__unknown_006f1080
                      ((void *)(*(int *)((int)this + 0x94) + 0xe8),param_2,abStack_18);
    uVar2 = (**(code **)(iVar1 + 0xc))(uVar3,uVar4,uVar2);
    uVar2 = (**(code **)(iVar1 + 0x94))(uVar2);
    return uVar2;
  case 1:
    uVar2 = A0x45c6b2f3_FDecalMaterial__unknown_006f1110
                      ((void *)(*(int *)((int)this + 0x94) + 0x10c),param_2,0x3f800000);
    return uVar2;
  case 2:
    uVar2 = A0x45c6b2f3_FDecalMaterial__unknown_006f1110
                      ((void *)(*(int *)((int)this + 0x94) + 0x130),param_2,0x3f800000);
    return uVar2;
  case 3:
    uStack_34 = 0;
    uVar2 = FUN_006f11d0((void *)(*(int *)((int)this + 0x94) + 0x158),param_2,&uStack_34);
    return uVar2;
  case 4:
    iVar1 = *param_2;
    uStack_1c = 0xffffffff;
    uVar2 = A0x45c6b2f3_FDecalMaterial__unknown_006f1080
                      ((void *)(*(int *)((int)this + 0x94) + 0x1cc),param_2,(byte *)&uStack_1c);
    uVar4 = 0xf;
    uVar3 = A0x45c6b2f3_FDecalMaterial__unknown_006f1110
                      ((void *)(*(int *)((int)this + 0x94) + 0x1a8),param_2,0);
    uVar2 = (**(code **)(iVar1 + 0xc))(uVar3,uVar4,uVar2);
    uVar2 = (**(code **)(iVar1 + 0x9c))(uVar2);
    return uVar2;
  case 5:
    iVar1 = *param_2;
    uStack_40 = 0xff808080;
    uVar2 = (**(code **)(iVar1 + 0x1c))(0x3f800000,uVar2);
    uVar2 = (**(code **)(iVar1 + 0x98))(uVar2);
    uVar4 = 4;
    uVar3 = A0x45c6b2f3_FDecalMaterial__unknown_006f1080
                      ((void *)(*(int *)((int)this + 0x94) + 0x50),param_2,local_4c);
    uVar2 = (**(code **)(iVar1 + 0xc))(uVar3,uVar4,uVar2);
    uVar2 = (**(code **)(iVar1 + 0x9c))(uVar2);
    return uVar2;
  case 6:
    abStack_3c[0] = 0x80;
    abStack_3c[1] = 0x80;
    abStack_3c[2] = 0x80;
    abStack_3c[3] = 0xff;
    uVar2 = A0x45c6b2f3_FDecalMaterial__unknown_006f1080
                      ((void *)(*(int *)((int)this + 0x94) + 0x74),param_2,abStack_3c);
    return uVar2;
  case 7:
    uVar2 = A0x45c6b2f3_FDecalMaterial__unknown_006f1110
                      ((void *)(*(int *)((int)this + 0x94) + 0x98),param_2,DAT_01834eb8);
    return uVar2;
  case 8:
    uVar2 = FUN_006f1160((void *)(*(int *)((int)this + 0x94) + 0xbc),param_2,
                         (undefined4 *)&stack0xffffffd4);
    iVar1 = *param_2;
    uVar3 = (**(code **)(iVar1 + 0xc4))(uVar2,1,1,0,0);
    uVar3 = (**(code **)(*param_2 + 100))(1,uVar3);
    uVar3 = (**(code **)(iVar1 + 0xa4))(uVar3);
    iVar1 = *param_2;
    uVar4 = (**(code **)(iVar1 + 0xc4))(uVar2,1,1,0,0);
    uVar4 = (**(code **)(*param_2 + 100))(2,uVar4);
    uVar4 = (**(code **)(iVar1 + 0xa4))(uVar4);
    uVar2 = (**(code **)(*param_2 + 0xc4))(uVar2,0,0,1,0);
    iVar1 = *param_2;
    uVar2 = (**(code **)(iVar1 + 200))(uVar3,uVar4,uVar2);
    uVar2 = (**(code **)(iVar1 + 200))(uVar2);
    return uVar2;
  case 9:
    abStack_3c[4] = 0;
    abStack_3c[5] = 0;
    abStack_3c[6] = 0;
    abStack_3c[7] = 0xff;
    uVar2 = A0x45c6b2f3_FDecalMaterial__unknown_006f1080
                      ((void *)(*(int *)((int)this + 0x94) + 0x184),param_2,abStack_3c + 4);
    return uVar2;
  default:
    return 0xffffffff;
  }
}




/* ========== A0x4e027f86.c ========== */

/*
 * SGW.exe - A0x4e027f86 (3 functions)
 */

/* [VTable] A0x4e027f86__00___FGFxPixelShader virtual function #8
   VTable: vtable__A0x4e027f86__00___FGFxPixelShader at 01961a3c
   Also in vtable: A0x4e027f86__01___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__03___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__07___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0BA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0CA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0EA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0IA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0BAA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0CAA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0EAA___FGFxPixelShader (slot 8)
   Also in vtable: A0x4e027f86__0IAA___FGFxPixelShader (slot 8) */

void __thiscall A0x4e027f86__00___FGFxPixelShader__vfunc_8(void *this,int *param_1)

{
  FShader__vfunc_8(this,param_1);
  FUN_005401e0(param_1,(int)this + 0x5c);
  FUN_005401e0(param_1,(int)this + 0x60);
  FUN_005401e0(param_1,(int)this + 100);
  FUN_005401e0(param_1,(int)this + 0x68);
  FUN_005401e0(param_1,(int)this + 0x6c);
  FUN_005401e0(param_1,(int)this + 0x70);
  return;
}


/* [VTable] A0x4e027f86__0BAAA___FGFxVertexShader virtual function #8
   VTable: vtable__A0x4e027f86__0BAAA___FGFxVertexShader at 01961eb4
   Also in vtable: A0x4e027f86__0CAAA___FGFxVertexShader (slot 8)
   Also in vtable: A0x4e027f86__0EAAA___FGFxVertexShader (slot 8)
   Also in vtable: A0x4e027f86__0IAAA___FGFxVertexShader (slot 8)
   Also in vtable: A0x4e027f86__0BAAAA___FGFxVertexShader (slot 8)
   Also in vtable: A0x4e027f86__0CAAAA___FGFxVertexShader (slot 8)
   Also in vtable: A0x4e027f86__0EAAAA___FGFxVertexShader (slot 8) */

void __thiscall A0x4e027f86__0BAAA___FGFxVertexShader__vfunc_8(void *this,int *param_1)

{
  FShader__vfunc_8(this,param_1);
  FUN_005401e0(param_1,(int)this + 0x5c);
  FUN_005401e0(param_1,(int)this + 0x60);
  FUN_005401e0(param_1,(int)this + 100);
  return;
}


/* [VTable] A0x4e027f86__00___FGFxPixelShader virtual function #7
   VTable: vtable__A0x4e027f86__00___FGFxPixelShader at 01961a3c
   Also in vtable: A0x4e027f86__01___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__03___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__07___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0BA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0CA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0EA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0IA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0BAA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0CAA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0EAA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0IAA___FGFxPixelShader (slot 7)
   Also in vtable: A0x4e027f86__0BAAA___FGFxVertexShader (slot 7)
   Also in vtable: A0x4e027f86__0CAAA___FGFxVertexShader (slot 7)
   Also in vtable: A0x4e027f86__0EAAA___FGFxVertexShader (slot 7)
   Also in vtable: A0x4e027f86__0IAAA___FGFxVertexShader (slot 7)
   Also in vtable: A0x4e027f86__0BAAAA___FGFxVertexShader (slot 7)
   Also in vtable: A0x4e027f86__0CAAAA___FGFxVertexShader (slot 7)
   Also in vtable: A0x4e027f86__0EAAAA___FGFxVertexShader (slot 7) */

undefined4 * __thiscall A0x4e027f86__00___FGFxPixelShader__vfunc_7(void *this,byte param_1)

{
  uint uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016db198;
  local_c = ExceptionList;
  uVar1 = DAT_01e7f9a0 ^ (uint)&stack0xffffffec;
  ExceptionList = &local_c;
  local_4 = 0xffffffff;
  FUN_00540350(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this,uVar1);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== A0x4e027f86_FGFxVertexShaderInterface.c ========== */

/*
 * SGW.exe - A0x4e027f86_FGFxVertexShaderInterface (1 functions)
 */

/* [VTable] A0x4e027f86_FGFxVertexShaderInterface virtual function #8
   VTable: vtable__A0x4e027f86_FGFxVertexShaderInterface at 01961738 */

void A0x4e027f86_FGFxVertexShaderInterface__vfunc_8(void)

{
  code *pcVar1;
  
  pcVar1 = (code *)swi(3);
  (*pcVar1)();
  return;
}




/* ========== Ability.c ========== */

/*
 * SGW.exe - Ability (1 functions)
 */

undefined4 * __thiscall Ability__vfunc_0(void *this,byte param_1)

{
  FUN_00d2a1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== AutoConfig.c ========== */

/*
 * SGW.exe - AutoConfig (1 functions)
 */

undefined4 * __thiscall AutoConfig__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = AutoConfig::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BehaviorEventData.c ========== */

/*
 * SGW.exe - BehaviorEventData (1 functions)
 */

undefined4 * __thiscall BehaviorEventData__vfunc_0(void *this,byte param_1)

{
  FUN_00d31950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== BinSection.c ========== */

/*
 * SGW.exe - BinSection (14 functions)
 */

/* [VTable] BinSection virtual function #12
   VTable: vtable_BinSection at 017fd4cc */

undefined4 __fastcall BinSection__vfunc_12(int param_1)

{
  if (*(int *)(param_1 + 0x24) != 0) {
    return *(undefined4 *)(*(int *)(param_1 + 0x24) + 0xc);
  }
  return 0;
}


/* [VTable] BinSection virtual function #15
   VTable: vtable_BinSection at 017fd4cc */

void __thiscall BinSection__vfunc_15(void *this,undefined4 *param_1)

{
  undefined4 *puVar1;
  int iVar2;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167bca8;
  local_c = ExceptionList;
  local_4 = 0;
  puVar1 = *(undefined4 **)((int)this + 0x3c);
  ExceptionList = &local_c;
  if (puVar1 != param_1) {
    ExceptionList = &local_c;
    *(undefined4 **)((int)this + 0x3c) = param_1;
    if (param_1 != (undefined4 *)0x0) {
      FUN_00457e40((int)param_1);
    }
    if (puVar1 != (undefined4 *)0x0) {
      iVar2 = FUN_00457e50((int)puVar1);
      if (iVar2 == 1) {
        (**(code **)*puVar1)(1);
      }
    }
  }
  local_4 = 0xffffffff;
  FUN_004585a0((int *)&param_1);
  ExceptionList = local_c;
  return;
}


/* [VTable] BinSection virtual function #13
   VTable: vtable_BinSection at 017fd4cc */

uint __thiscall BinSection__vfunc_13(void *this,void *param_1)

{
  int *piVar1;
  uint uVar2;
  void *unaff_ESI;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167d7c0;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  if (*(int *)((int)param_1 + 0x14) != 0) {
    ExceptionList = &local_c;
    param_1 = (void *)FUN_004693d0(param_1,(undefined4 *)((int)this + 0x3c),(void *)((int)this + 8))
    ;
    if ((char)param_1 == '\0') goto LAB_00472629;
  }
  piVar1 = *(int **)((int)this + 0x3c);
  if (piVar1 != (int *)0x0) {
    FUN_00457e40((int)this);
    local_4 = 0xffffffff;
    uVar2 = (**(code **)(*piVar1 + 0x38))();
    ExceptionList = unaff_ESI;
    return uVar2;
  }
LAB_00472629:
  ExceptionList = local_c;
  return (uint)param_1 & 0xffffff00;
}


/* [VTable] BinSection virtual function #11
   VTable: vtable_BinSection at 017fd4cc */

void * __thiscall BinSection__vfunc_11(void *this,void *param_1)

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
  FUN_00437710(param_1,(void *)((int)this + 8),0,0xffffffff);
  ExceptionList = pvStack_c;
  return param_1;
}


/* Also in vtable: BinSection (slot 0) */

undefined4 * __thiscall BinSection__vfunc_0(void *this,byte param_1)

{
  FUN_00472850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] BinSection virtual function #10
   VTable: vtable_BinSection at 017fd4cc */

void __fastcall BinSection__vfunc_10(int param_1)

{
  void *this;
  int *piVar1;
  int *piVar2;
  undefined4 *puVar3;
  int iVar4;
  undefined4 *local_24;
  void *local_20;
  undefined1 *local_1c;
  undefined1 *local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167d8ab;
  local_c = ExceptionList;
  this = (void *)(param_1 + 0x2c);
  ExceptionList = &local_c;
  *(undefined1 *)(param_1 + 0x28) = 1;
  piVar1 = *(int **)(param_1 + 0x34);
  if (piVar1 < *(int **)(param_1 + 0x30)) {
    _invalid_parameter_noinfo();
  }
  piVar2 = *(int **)(param_1 + 0x30);
  if (*(int **)(param_1 + 0x34) < piVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0046f260(this,(int *)local_18,(int)this,piVar2,(int)this,piVar1);
  local_20 = (void *)scalable_malloc();
  if (local_20 == (void *)0x0) {
    FUN_004099f0((exception *)local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_1c = &stack0xffffffc8;
  local_18[0] = &stack0xffffffc8;
  local_4 = 0;
  local_24 = FUN_00472290(local_20,(void *)0x0,0,0);
  local_4 = 0xffffffff;
  if (local_24 != (undefined4 *)0x0) {
    FUN_00457e40((int)local_24);
  }
  local_4 = 4;
  puVar3 = *(undefined4 **)(param_1 + 0x24);
  if (puVar3 != local_24) {
    *(undefined4 **)(param_1 + 0x24) = local_24;
    if (local_24 != (undefined4 *)0x0) {
      FUN_00457e40((int)local_24);
    }
    if (puVar3 != (undefined4 *)0x0) {
      iVar4 = FUN_00457e50((int)puVar3);
      if (iVar4 == 1) {
        (**(code **)*puVar3)();
      }
    }
  }
  local_4 = 0xffffffff;
  FUN_004585a0((int *)&local_24);
  ExceptionList = local_c;
  return;
}


/* [VTable] BinSection virtual function #41
   VTable: vtable_BinSection at 017fd4cc */

undefined4 __thiscall BinSection__vfunc_41(void *this,undefined4 *param_1)

{
  undefined4 *puVar1;
  int *piVar2;
  int *piVar3;
  int iVar4;
  undefined4 uVar5;
  void *this_00;
  int local_14 [2];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167bca8;
  local_c = ExceptionList;
  local_4 = 0;
  puVar1 = *(undefined4 **)((int)this + 0x24);
  ExceptionList = &local_c;
  if (puVar1 != param_1) {
    ExceptionList = &local_c;
    *(undefined4 **)((int)this + 0x24) = param_1;
    if (param_1 != (undefined4 *)0x0) {
      FUN_00457e40((int)param_1);
    }
    if (puVar1 != (undefined4 *)0x0) {
      iVar4 = FUN_00457e50((int)puVar1);
      if (iVar4 == 1) {
        (**(code **)*puVar1)(1);
      }
    }
  }
  *(undefined1 *)((int)this + 0x28) = 0;
  piVar2 = *(int **)((int)this + 0x34);
  this_00 = (void *)((int)this + 0x2c);
  if (piVar2 < *(int **)((int)this + 0x30)) {
    _invalid_parameter_noinfo();
  }
  piVar3 = *(int **)((int)this + 0x30);
  if (*(int **)((int)this + 0x34) < piVar3) {
    _invalid_parameter_noinfo();
  }
  FUN_0046f260(this_00,local_14,(int)this_00,piVar3,(int)this_00,piVar2);
  local_4 = 0xffffffff;
  uVar5 = FUN_004585a0((int *)&param_1);
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)uVar5 >> 8),1);
}


/* [VTable] BinSection virtual function #1
   VTable: vtable_BinSection at 017fd4cc */

int __fastcall BinSection__vfunc_1(int param_1)

{
  if (*(char *)(param_1 + 0x28) == '\0') {
    BinSection__unknown_00472ba0(param_1);
  }
  if (*(int *)(param_1 + 0x30) == 0) {
    return 0;
  }
  return *(int *)(param_1 + 0x34) - *(int *)(param_1 + 0x30) >> 2;
}


/* [VTable] BinSection virtual function #3
   VTable: vtable_BinSection at 017fd4cc */

undefined4 * __thiscall BinSection__vfunc_3(void *this,undefined4 *param_1,uint param_2)

{
  int iVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167d9b1;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  if (*(char *)((int)this + 0x28) == '\0') {
    ExceptionList = &local_c;
    BinSection__unknown_00472ba0((int)this);
  }
  iVar1 = *(int *)((int)this + 0x30);
  if ((iVar1 != 0) && (param_2 < (uint)(*(int *)((int)this + 0x34) - iVar1 >> 2))) {
    if ((iVar1 == 0) || ((uint)(*(int *)((int)this + 0x34) - iVar1 >> 2) <= param_2)) {
      _invalid_parameter_noinfo();
    }
    FUN_00458600(param_1,(int *)(*(int *)((int)this + 0x30) + param_2 * 4));
    ExceptionList = local_c;
    return param_1;
  }
  *param_1 = 0;
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] BinSection virtual function #5
   VTable: vtable_BinSection at 017fd4cc */

int * __thiscall BinSection__vfunc_5(void *this,int *param_1,void *param_2)

{
  void *this_00;
  void *this_01;
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  int local_4;
  
  puStack_8 = &LAB_0167da17;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  if (*(char *)((int)this + 0x28) == '\0') {
    ExceptionList = &local_c;
    BinSection__unknown_00472ba0((int)this);
  }
  this_00 = (void *)scalable_malloc();
  if (this_00 == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4 = 1;
  this_01 = (void *)scalable_malloc();
  if (this_01 == (void *)0x0) {
    FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
  }
  local_4._1_3_ = (uint3)((uint)local_4 >> 8);
  local_4._0_1_ = 2;
  puVar1 = FUN_00472290(this_01,(void *)0x0,0,0);
  local_4._0_1_ = 1;
  if (puVar1 != (undefined4 *)0x0) {
    FUN_00457e40((int)puVar1);
  }
  local_4._0_1_ = 1;
  puVar1 = FUN_00472760(this_00,param_2,(int)puVar1);
  local_4 = (uint)local_4._1_3_ << 8;
  *param_1 = (int)puVar1;
  if (puVar1 != (undefined4 *)0x0) {
    FUN_00457e40((int)puVar1);
  }
  local_4 = 0;
  FUN_0046a880((void *)((int)this + 0x2c),param_1);
  ExceptionList = local_c;
  return param_1;
}


/* WARNING: Removing unreachable block (ram,0x00473657) */
/* [VTable] BinSection virtual function #7
   VTable: vtable_BinSection at 017fd4cc */

int * __thiscall BinSection__vfunc_7(void *this,int *param_1,int param_2)

{
  uint uVar1;
  int iVar2;
  char *pcVar3;
  uint uVar4;
  uint uVar5;
  char *pcVar6;
  uint uVar7;
  bool bVar8;
  char acStack_2d [9];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167da61;
  local_c = ExceptionList;
  uVar7 = 0;
  acStack_2d[1] = '\0';
  acStack_2d[2] = '\0';
  acStack_2d[3] = '\0';
  acStack_2d[4] = '\0';
  ExceptionList = &local_c;
  if (*(char *)((int)this + 0x28) == '\0') {
    ExceptionList = &local_c;
    BinSection__unknown_00472ba0((int)this);
  }
  while( true ) {
    iVar2 = *(int *)((int)this + 0x30);
    if ((iVar2 == 0) || ((uint)(*(int *)((int)this + 0x34) - iVar2 >> 2) <= uVar7)) {
      *param_1 = 0;
      ExceptionList = local_c;
      return param_1;
    }
    if ((iVar2 == 0) || ((uint)(*(int *)((int)this + 0x34) - iVar2 >> 2) <= uVar7)) {
      _invalid_parameter_noinfo();
    }
    iVar2 = (**(code **)(**(int **)(*(int *)((int)this + 0x30) + uVar7 * 4) + 0x2c))(acStack_2d + 5)
    ;
    local_4 = 1;
    uVar4 = *(uint *)(param_2 + 0x14);
    if (*(uint *)(param_2 + 0x18) < 0x10) {
      pcVar6 = (char *)(param_2 + 4);
    }
    else {
      pcVar6 = *(char **)(param_2 + 4);
    }
    uVar1 = *(uint *)(iVar2 + 0x14);
    uVar5 = uVar1;
    if (uVar4 <= uVar1) {
      uVar5 = uVar4;
    }
    if (*(uint *)(iVar2 + 0x18) < 0x10) {
      pcVar3 = (char *)(iVar2 + 4);
    }
    else {
      pcVar3 = *(char **)(iVar2 + 4);
    }
    iVar2 = std::char_traits<char>::compare(pcVar3,pcVar6,uVar5);
    bVar8 = false;
    if (iVar2 == 0) {
      if (uVar1 < uVar4) {
        uVar4 = 0xffffffff;
      }
      else {
        uVar4 = (uint)(uVar1 != uVar4);
      }
      bVar8 = uVar4 == 0;
    }
    local_4 = local_4 & 0xffffff00;
    if (0xf < uStack_10) break;
    uStack_10 = 0xf;
    acStack_2d[0] = '\0';
    uStack_14 = 0;
    std::char_traits<char>::assign((char *)auStack_24,acStack_2d);
    if (bVar8) {
      if ((*(int *)((int)this + 0x30) == 0) ||
         ((uint)(*(int *)((int)this + 0x34) - *(int *)((int)this + 0x30) >> 2) <= uVar7)) {
        _invalid_parameter_noinfo();
      }
      iVar2 = *(int *)(*(int *)((int)this + 0x30) + uVar7 * 4);
      *param_1 = iVar2;
      if (iVar2 != 0) {
        FUN_00457e40(iVar2);
      }
      ExceptionList = local_c;
      return param_1;
    }
    uVar7 = uVar7 + 1;
  }
                    /* WARNING: Subroutine does not return */
  scalable_free(auStack_24[0]);
}


/* WARNING: Removing unreachable block (ram,0x00473807) */
/* [VTable] BinSection virtual function #9
   VTable: vtable_BinSection at 017fd4cc */

void __thiscall BinSection__vfunc_9(void *this,int param_1)

{
  int *piVar1;
  uint uVar2;
  int iVar3;
  char *pcVar4;
  uint uVar5;
  uint uVar6;
  char *pcVar7;
  uint uVar8;
  bool bVar9;
  char cStack_2d;
  uint uStack_2c;
  undefined1 local_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d5d28;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  if (*(char *)((int)this + 0x28) == '\0') {
    ExceptionList = &local_c;
    BinSection__unknown_00472ba0((int)this);
  }
  uVar8 = 0;
  while( true ) {
    iVar3 = *(int *)((int)this + 0x30);
    if (iVar3 == 0) {
      ExceptionList = local_c;
      return;
    }
    if ((uint)(*(int *)((int)this + 0x34) - iVar3 >> 2) <= uVar8) break;
    if ((iVar3 == 0) || ((uint)(*(int *)((int)this + 0x34) - iVar3 >> 2) <= uVar8)) {
      _invalid_parameter_noinfo();
    }
    iVar3 = (**(code **)(**(int **)(*(int *)((int)this + 0x30) + uVar8 * 4) + 0x2c))(local_28);
    uStack_4 = 0;
    uVar5 = *(uint *)(param_1 + 0x14);
    if (*(uint *)(param_1 + 0x18) < 0x10) {
      pcVar7 = (char *)(param_1 + 4);
    }
    else {
      pcVar7 = *(char **)(param_1 + 4);
    }
    uVar2 = *(uint *)(iVar3 + 0x14);
    uVar6 = uVar2;
    if (uVar5 <= uVar2) {
      uVar6 = uVar5;
    }
    if (*(uint *)(iVar3 + 0x18) < 0x10) {
      pcVar4 = (char *)(iVar3 + 4);
    }
    else {
      pcVar4 = *(char **)(iVar3 + 4);
    }
    iVar3 = std::char_traits<char>::compare(pcVar4,pcVar7,uVar6);
    bVar9 = false;
    if (iVar3 == 0) {
      if (uVar2 < uVar5) {
        uVar5 = 0xffffffff;
      }
      else {
        uVar5 = (uint)(uVar2 != uVar5);
      }
      bVar9 = uVar5 == 0;
    }
    uStack_4 = 0xffffffff;
    if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_24[0]);
    }
    uStack_10 = 0xf;
    cStack_2d = '\0';
    uStack_14 = 0;
    std::char_traits<char>::assign((char *)auStack_24,&cStack_2d);
    if (bVar9) {
      uVar5 = *(uint *)((int)this + 0x30);
      if (*(uint *)((int)this + 0x34) < uVar5) {
        _invalid_parameter_noinfo();
      }
      piVar1 = (int *)(uVar5 + uVar8 * 4);
      if ((*(int **)((int)this + 0x34) < piVar1) || (piVar1 < *(int **)((int)this + 0x30))) {
        _invalid_parameter_noinfo();
      }
      uStack_2c = uStack_2c & 0xffffff00;
      FUN_0046bde0(piVar1 + 1,*(int **)((int)this + 0x34),piVar1);
      VisualChecker__unknown_004625a0(*(int **)((int)this + 0x34) + -1,*(int **)((int)this + 0x34));
      *(int *)((int)this + 0x34) = *(int *)((int)this + 0x34) + -4;
      ExceptionList = local_c;
      return;
    }
    uVar8 = uVar8 + 1;
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] BinSection virtual function #8
   VTable: vtable_BinSection at 017fd4cc */

void __thiscall BinSection__vfunc_8(void *this,int param_1)

{
  int *piVar1;
  int *piVar2;
  int *piVar3;
  code *pcVar4;
  void *this_00;
  undefined4 local_14;
  int *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167bca8;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  if (*(char *)((int)this + 0x28) == '\0') {
    ExceptionList = &local_c;
    BinSection__unknown_00472ba0((int)this);
  }
  pcVar4 = _invalid_parameter_noinfo_exref;
  piVar1 = *(int **)((int)this + 0x34);
  this_00 = (void *)((int)this + 0x2c);
  if (piVar1 < *(int **)((int)this + 0x30)) {
    _invalid_parameter_noinfo();
  }
  piVar2 = *(int **)((int)this + 0x30);
  piVar3 = piVar2;
  if (*(int **)((int)this + 0x34) < piVar2) {
    _invalid_parameter_noinfo();
  }
  for (; (piVar3 != piVar1 && (*piVar3 != param_1)); piVar3 = piVar3 + 1) {
  }
  piVar1 = *(int **)((int)this + 0x34);
  local_10 = piVar2;
  if (piVar1 < *(int **)((int)this + 0x30)) {
    _invalid_parameter_noinfo();
    pcVar4 = _invalid_parameter_noinfo_exref;
  }
  if (this_00 == (void *)0x0) {
    (*pcVar4)();
  }
  if (piVar3 != piVar1) {
    FUN_0046f1f0(this_00,&local_14,this_00,piVar3);
  }
  local_4 = 0xffffffff;
  FUN_004585a0(&param_1);
  ExceptionList = local_c;
  return;
}


/* [VTable] BinSection virtual function #28
   VTable: vtable_BinSection at 017fd4cc */

int * __thiscall BinSection__vfunc_28(void *this,int *param_1)

{
  int iVar1;
  int iStack_1c;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167da99;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (((*(char *)((int)this + 0x28) != '\0') &&
      (ExceptionList = &pvStack_c, *(int *)((int)this + 0x30) != 0)) &&
     (iStack_1c = *(int *)((int)this + 0x34) - *(int *)((int)this + 0x30) >> 2,
     ExceptionList = &pvStack_c, iStack_1c != 0)) {
    ExceptionList = &pvStack_c;
    BWResource__unknown_00472e10(this,&iStack_1c);
    local_4 = 0;
    (**(code **)(*(int *)this + 0xa4))();
    iStack_1c = 0x473a39;
    BinSection__unknown_00472ba0((int)this);
  }
  iVar1 = *(int *)((int)this + 0x24);
  *param_1 = iVar1;
  if (iVar1 != 0) {
    iStack_1c = 0x473a4b;
    FUN_00457e40(iVar1);
  }
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== CMETextCmds.c ========== */

/*
 * SGW.exe - CMETextCmds (12 functions)
 */

/* [VTable] CMETextCmds___cmd__CommandList_sequence virtual function #1
   VTable: vtable_CMETextCmds___cmd__CommandList_sequence at 01926cd0 */

void __fastcall CMETextCmds___cmd__CommandList_sequence__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  return;
}


/* [VTable] CMETextCmds__cmd__CommandList virtual function #1
   VTable: vtable_CMETextCmds__cmd__CommandList at 019250c4 */

void __thiscall CMETextCmds__cmd__CommandList__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] CMETextCmds__cmd__CommandList virtual function #2
   VTable: vtable_CMETextCmds__cmd__CommandList at 019250c4 */

void __thiscall CMETextCmds__cmd__CommandList__vfunc_2(void *this,undefined4 param_1)

{
  int iVar1;
  
  if ((*(int *)((int)this + 8) != 0) && (iVar1 = 0, 0 < *(int *)((int)this + 4))) {
    do {
      (**(code **)(*(int *)(*(int *)((int)this + 8) + iVar1 * 8) + 8))(param_1);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + 4));
  }
  return;
}


/* [VTable] CMETextCmds___cmd__CommandList_sequence virtual function #2
   VTable: vtable_CMETextCmds___cmd__CommandList_sequence at 01926cd0 */

void __thiscall CMETextCmds___cmd__CommandList_sequence__vfunc_2(void *this,int param_1)

{
  int iVar1;
  
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 4),0xc);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 4) + 8))(param_1);
  }
  return;
}


/* [VTable] CMETextCmds___cmd__CommandList_sequence virtual function #4
   VTable: vtable_CMETextCmds___cmd__CommandList_sequence at 01926cd0 */

undefined4 __thiscall CMETextCmds___cmd__CommandList_sequence__vfunc_4(void *this,uint param_1)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a4b620
                    (param_1,"Command",-1,*(uint *)((int)this + 4),(uint *)0x0,0,&DAT_0192670f,0xc);
  if (-1 < iVar1) {
    (**(code **)(**(int **)((int)this + 4) + 0x10))(param_1,"Command",iVar1,&DAT_0192670f);
  }
  return 0;
}


/* [VTable] CMETextCmds___cmd__CommandList_sequence virtual function #6
   VTable: vtable_CMETextCmds___cmd__CommandList_sequence at 01926cd0 */

void __thiscall
CMETextCmds___cmd__CommandList_sequence__vfunc_6
          (void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  CMETextCmds__unknown_00a61da0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] CMETextCmds__cmd__CommandList virtual function #6
   VTable: vtable_CMETextCmds__cmd__CommandList at 019250c4 */

void __thiscall
CMETextCmds__cmd__CommandList__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CMETextCmds__unknown_00a61ed0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] CMETextCmds___cmd__CommandList_sequence virtual function #5
   VTable: vtable_CMETextCmds___cmd__CommandList_sequence at 01926cd0 */

int __thiscall
CMETextCmds___cmd__CommandList_sequence__vfunc_5
          (void *this,char *param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  
  iVar1 = CMETextCmds__unknown_00a61da0(param_1,param_2,this,param_3);
  if (iVar1 != 0) {
    CMETextCmds_cmd__unknown_00a63340(param_1);
  }
  return iVar1;
}


/* [VTable] CMETextCmds__cmd__CommandList virtual function #5
   VTable: vtable_CMETextCmds__cmd__CommandList at 019250c4 */

int * __thiscall
CMETextCmds__cmd__CommandList__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CMETextCmds__unknown_00a61ed0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    CMETextCmds_cmd__unknown_00a63340(param_1);
  }
  return piVar1;
}


/* [VTable] CMETextCmds___cmd__CommandList_sequence virtual function #7
   VTable: vtable_CMETextCmds___cmd__CommandList_sequence at 01926cd0 */

undefined4 * __thiscall CMETextCmds___cmd__CommandList_sequence__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = CMETextCmds::__cmd__CommandList_sequence::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,8,*(int *)((int)this + -4),FUN_00a5f2c0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] CMETextCmds__cmd__CommandList virtual function #7
   VTable: vtable_CMETextCmds__cmd__CommandList at 019250c4 */

undefined4 * __thiscall CMETextCmds__cmd__CommandList__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = CMETextCmds::_cmd__CommandList::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x10,*(int *)((int)this + -4),FUN_00a4bfb0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* Also in vtable: CMETextCmds___cmd__CommandList_sequence (slot 0) */

undefined4 CMETextCmds___cmd__CommandList_sequence__vfunc_0(void)

{
  return 0x15;
}




/* ========== CMETextCmds_cmd.c ========== */

/*
 * SGW.exe - CMETextCmds_cmd (14 functions)
 */

/* Also in vtable: SGWBindableActions__action__actionGroups (slot 0)
   Also in vtable: CMETextCmds_cmd__MandatoryParamType (slot 0)
   Also in vtable: SGWUIPersistence_gui__WindowStateType (slot 0)
   Also in vtable: SGWSystemOptions_SGWSystemOptions__OptionSaveType (slot 0) */

undefined4 CMETextCmds_cmd__MandatoryParamType__vfunc_0(void)

{
  return 10;
}


/* [VTable] CMETextCmds_cmd__OptionalParamType virtual function #1
   VTable: vtable_CMETextCmds_cmd__OptionalParamType at 01926cac */

void __thiscall CMETextCmds_cmd__OptionalParamType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x10) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  return;
}


/* [VTable] CMETextCmds_cmd__OptionalParamType virtual function #2
   VTable: vtable_CMETextCmds_cmd__OptionalParamType at 01926cac */

void CMETextCmds_cmd__OptionalParamType__vfunc_2(void)

{
  return;
}


/* [VTable] CMETextCmds_cmd__MandatoryParamType virtual function #1
   VTable: vtable_CMETextCmds_cmd__MandatoryParamType at 01926c88 */

void __thiscall CMETextCmds_cmd__MandatoryParamType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x10) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  return;
}


/* [VTable] CMETextCmds_cmd__MandatoryParamType virtual function #2
   VTable: vtable_CMETextCmds_cmd__MandatoryParamType at 01926c88 */

void CMETextCmds_cmd__MandatoryParamType__vfunc_2(void)

{
  return;
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #3
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

undefined4 __thiscall
CMETextCmds_cmd__CommandType__vfunc_3(void *this,uint param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  undefined4 uVar2;
  
  iVar1 = FUN_00a40f40(param_1,(uint)this,(uint *)0x0,0,param_2,0xc);
  iVar1 = (**(code **)(*(int *)this + 0x10))(param_1,param_2,iVar1,param_3);
  if (iVar1 != 0) {
    return *(undefined4 *)(param_1 + 0x16138);
  }
  uVar2 = CMETextCmds_cmd__unknown_00a602b0(param_1);
  return uVar2;
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #2
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

void __thiscall CMETextCmds_cmd__CommandType__vfunc_2(void *this,int param_1)

{
  FUN_00a60a10(param_1,(int)this + 4);
  FUN_00a608f0(param_1,(int)this + 0x14);
  return;
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #1
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

void __thiscall CMETextCmds_cmd__CommandType__vfunc_1(void *this,undefined4 param_1)

{
  void *pvVar1;
  void *pvVar2;
  void *pvVar3;
  int local_8 [2];
  
  pvVar1 = (void *)((int)this + 4);
  *(undefined4 *)((int)this + 0x38) = param_1;
  pvVar2 = *(void **)((int)this + 0xc);
  if (pvVar2 < *(void **)((int)this + 8)) {
    _invalid_parameter_noinfo();
  }
  pvVar3 = *(void **)((int)this + 8);
  if (*(void **)((int)this + 0xc) < pvVar3) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar3,(int)pvVar1,pvVar2);
  pvVar2 = *(void **)((int)this + 0x1c);
  pvVar1 = (void *)((int)this + 0x14);
  if (pvVar2 < *(void **)((int)this + 0x18)) {
    _invalid_parameter_noinfo();
  }
  pvVar3 = *(void **)((int)this + 0x18);
  if (*(void **)((int)this + 0x1c) < pvVar3) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar3,(int)pvVar1,pvVar2);
  *(undefined4 *)((int)this + 0x24) = 0;
  *(undefined4 *)((int)this + 0x28) = 0;
  *(undefined4 *)((int)this + 0x2c) = 0;
  *(undefined4 *)((int)this + 0x30) = 0;
  *(undefined4 *)((int)this + 0x34) = 0;
  return;
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #6
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

void __thiscall
CMETextCmds_cmd__CommandType__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00a62bb0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #5
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

int * __thiscall
CMETextCmds_cmd__CommandType__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00a62bb0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    CMETextCmds_cmd__unknown_00a63340(param_1);
  }
  return piVar1;
}


/* [VTable] CMETextCmds_cmd__OptionalParamType virtual function #7
   VTable: vtable_CMETextCmds_cmd__OptionalParamType at 01926cac */

undefined4 * __thiscall CMETextCmds_cmd__OptionalParamType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = CMETextCmds::cmd__OptionalParamType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x14,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00a5f2a0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] CMETextCmds_cmd__MandatoryParamType virtual function #7
   VTable: vtable_CMETextCmds_cmd__MandatoryParamType at 01926c88 */

undefined4 * __thiscall CMETextCmds_cmd__MandatoryParamType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = CMETextCmds::cmd__MandatoryParamType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x14,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00a5f270);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #7
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

undefined4 * __thiscall CMETextCmds_cmd__CommandType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00a63a70(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x3c,*(int *)((int)this + -4),FUN_00a63a70);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* Also in vtable: SGWSystemOptions__SGWSystemOptions__savedOptions (slot 0)
   Also in vtable: CMETextCmds_cmd__CommandType (slot 0)
   Also in vtable: SGWBindableActions__action__ActionGroupType_action (slot 0)
   Also in vtable: SGWTableOfContents_cmd__MandatoryParamType (slot 0) */

undefined4 CMETextCmds_cmd__CommandType__vfunc_0(void)

{
  return 0xc;
}




/* ========== CMEWinCrashReport_crash.c ========== */

/*
 * SGW.exe - CMEWinCrashReport_crash (3 functions)
 */

/* [VTable] CMEWinCrashReport_crash__CrashReport virtual function #1
   VTable: vtable_CMEWinCrashReport_crash__CrashReport at 01925a70 */

void __thiscall CMEWinCrashReport_crash__CrashReport__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x20) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined4 *)((int)this + 0x14) = 0;
  *(undefined4 *)((int)this + 0x18) = 0;
  *(undefined4 *)((int)this + 0x1c) = 0;
  return;
}


/* [VTable] CMEWinCrashReport_crash__CrashReport virtual function #2
   VTable: vtable_CMEWinCrashReport_crash__CrashReport at 01925a70 */

void __thiscall CMEWinCrashReport_crash__CrashReport__vfunc_2(void *this,int param_1)

{
  FUN_00a3b030(param_1,*(uint *)((int)this + 4),0xb);
  FUN_00a3b030(param_1,*(uint *)((int)this + 8),0xb);
  FUN_00a3b030(param_1,*(uint *)((int)this + 0xc),0xb);
  FUN_00a3b030(param_1,*(uint *)((int)this + 0x10),0xb);
  FUN_00a3b030(param_1,*(uint *)((int)this + 0x1c),0xb);
  return;
}


/* [VTable] CMEWinCrashReport_crash__CrashReport virtual function #7
   VTable: vtable_CMEWinCrashReport_crash__CrashReport at 01925a70 */

undefined4 * __thiscall CMEWinCrashReport_crash__CrashReport__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = CMEWinCrashReport::crash__CrashReport::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x24,*(int *)((int)this + -4),FUN_00a55a00);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}




/* ========== CMEWinCrashReport_crashapp.c ========== */

/*
 * SGW.exe - CMEWinCrashReport_crashapp (6 functions)
 */

/* [VTable] CMEWinCrashReport_crashapp__CrashFileEntry virtual function #1
   VTable: vtable_CMEWinCrashReport_crashapp__CrashFileEntry at 0192727c */

void __thiscall CMEWinCrashReport_crashapp__CrashFileEntry__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 8) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  return;
}


/* [VTable] CMEWinCrashReport_crashapp__CrashFileEntry virtual function #2
   VTable: vtable_CMEWinCrashReport_crashapp__CrashFileEntry at 0192727c */

void CMEWinCrashReport_crashapp__CrashFileEntry__vfunc_2(void)

{
  return;
}


/* [VTable] CMEWinCrashReport_crashapp__CrashFileList virtual function #2
   VTable: vtable_CMEWinCrashReport_crashapp__CrashFileList at 019272a0 */

void __thiscall CMEWinCrashReport_crashapp__CrashFileList__vfunc_2(void *this,int param_1)

{
  FUN_00a64420(param_1,(int)this + 4);
  return;
}


/* [VTable] CMEWinCrashReport_crashapp__CrashFileList virtual function #1
   VTable: vtable_CMEWinCrashReport_crashapp__CrashFileList at 019272a0 */

void __thiscall CMEWinCrashReport_crashapp__CrashFileList__vfunc_1(void *this,undefined4 param_1)

{
  void *this_00;
  void *pvVar1;
  void *pvVar2;
  int local_8 [2];
  
  this_00 = (void *)((int)this + 4);
  *(undefined4 *)((int)this + 0x18) = param_1;
  pvVar1 = *(void **)((int)this + 0xc);
  if (pvVar1 < *(void **)((int)this + 8)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 8);
  if (*(void **)((int)this + 0xc) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(this_00,local_8,(int)this_00,pvVar2,(int)this_00,pvVar1);
  *(undefined4 *)((int)this + 0x14) = 0;
  return;
}


/* [VTable] CMEWinCrashReport_crashapp__CrashFileEntry virtual function #7
   VTable: vtable_CMEWinCrashReport_crashapp__CrashFileEntry at 0192727c */

undefined4 * __thiscall CMEWinCrashReport_crashapp__CrashFileEntry__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = CMEWinCrashReport::crashapp__CrashFileEntry::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0xc,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00a63b70);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] CMEWinCrashReport_crashapp__CrashFileList virtual function #7
   VTable: vtable_CMEWinCrashReport_crashapp__CrashFileList at 019272a0 */

undefined4 * __thiscall CMEWinCrashReport_crashapp__CrashFileList__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00a66b80(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x1c,*(int *)((int)this + -4),FUN_00a66b80);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}




/* ========== CME_CME_Detail_HH_HU.c ========== */

/*
 * SGW.exe - CME_CME_Detail_HH_HU (1 functions)
 */

/* Also in vtable: CME_CME_Detail_HH_HU__DefaultCountTypeTraits___CountedBaseTempl (slot 0) */

undefined4 * __thiscall
CME_CME_Detail_HH_HU__DefaultCountTypeTraits___CountedBaseTempl__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_CoverNodeVariance.c ========== */

/*
 * SGW.exe - CME_CoverNodeVariance (1 functions)
 */

undefined4 * __thiscall CME_CoverNodeVariance__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CME::KMeans<class_CME::CoverNode*,class_Vector3>::VarianceFunctor::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_Detail_PropertyNode.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode (3 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode__N___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode__N___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<bool>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* Also in vtable: CME_Detail_PropertyNode__J___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode__J___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<__int64>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


undefined4 * __thiscall CME_Detail_PropertyNode__K___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<unsigned___int64>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_CME_PBVTextCmd.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_CME_PBVTextCmd (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_CME_PBVTextCmd___Property (slot 0) */

undefined4 * __thiscall
CME_Detail_PropertyNode_CME_PBVTextCmd___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<class_CME::TextCmd_const*>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_D.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_D (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_D___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode_D___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<char>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_E.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_E (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_E___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode_E___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<unsigned_char>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_F.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_F (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_F___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode_F___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<short>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_G.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_G (1 functions)
 */

undefined4 * __thiscall CME_Detail_PropertyNode_G___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<unsigned_short>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_H.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_H (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_H___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode_H___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<int>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_J.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_J (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_J___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode_J___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<long>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_K.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_K (1 functions)
 */

undefined4 * __thiscall CME_Detail_PropertyNode_K___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<unsigned_long>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_M.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_M (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_M___Property (slot 0) */

undefined4 * __thiscall CME_Detail_PropertyNode_M___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<float>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_VVector2.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_VVector2 (1 functions)
 */

undefined4 * __thiscall
CME_Detail_PropertyNode_VVector2___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<class_Vector2>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_VVector3.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_VVector3 (1 functions)
 */

/* Also in vtable: CME_Detail_PropertyNode_VVector3___Property (slot 0) */

undefined4 * __thiscall
CME_Detail_PropertyNode_VVector3___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<class_Vector3>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_Detail_PropertyNode_VVector4.c ========== */

/*
 * SGW.exe - CME_Detail_PropertyNode_VVector4 (1 functions)
 */

undefined4 * __thiscall
CME_Detail_PropertyNode_VVector4___Property__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01679400;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::Detail::PropertyNode::Property<class_Vector4>::vftable;
  *(undefined ***)this = CME::Detail::PropertyNode::PropertyBase::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_EventSignal_CME_VTEXT_CMD_TOO_FEW_PARAMS.c ========== */

/*
 * SGW.exe - CME_EventSignal_CME_VTEXT_CMD_TOO_FEW_PARAMS (1 functions)
 */

/* Also in vtable: CME_EventSignal_CME_VTEXT_CMD_TOO_FEW_PARAMS___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_CME_VTEXT_CMD_TOO_FEW_PARAMS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00a517b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_Callback.c ========== */

/*
 * SGW.exe - CME_EventSignal_Callback (1 functions)
 */

/* Also in vtable: CME_EventSignal_Callback (slot 0) */

undefined4 * __thiscall CME_EventSignal_Callback__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CME::EventSignal::Callback::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_EmitInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_EmitInfo (1 functions)
 */

/* Also in vtable: CME_EventSignal_EmitInfo (slot 0) */

undefined4 * __thiscall CME_EventSignal_EmitInfo__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CME::EventSignal::EmitInfo::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_JUCookedCharCreationData_U.c ========== */

/*
 * SGW.exe - CME_EventSignal_JUCookedCharCreationData_U (2 functions)
 */

/* Also in vtable:
   CME_EventSignal_JUCookedCharCreationData_U__Event_Cache_ElementError___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_JUCookedCharCreationData_U__Event_Cache_ElementError___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00422780(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable:
   CME_EventSignal_JUCookedCharCreationData_U__Event_Cache_ElementReady___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_JUCookedCharCreationData_U__Event_Cache_ElementReady___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00433490(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_JUCookedKismetEventSetData_U.c ========== */

/*
 * SGW.exe - CME_EventSignal_JUCookedKismetEventSetData_U (2 functions)
 */

/* Also in vtable:
   CME_EventSignal_JUCookedKismetEventSetData_U__Event_Cache_ElementError___TypedEmitInfo (slot 0)
    */

undefined4 * __thiscall
CME_EventSignal_JUCookedKismetEventSetData_U__Event_Cache_ElementError___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00422030(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable:
   CME_EventSignal_JUCookedKismetEventSetData_U__Event_Cache_ElementReady___TypedEmitInfo (slot 0)
    */

undefined4 * __thiscall
CME_EventSignal_JUCookedKismetEventSetData_U__Event_Cache_ElementReady___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00433130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_JVCookedKismetEventSequenceData_U.c ========== */

/*
 * SGW.exe - CME_EventSignal_JVCookedKismetEventSequenceData_U (2 functions)
 */

/* Also in vtable:
   CME_EventSignal_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError___TypedEmitInfo (slot
   0) */

undefined4 * __thiscall
CME_EventSignal_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00422100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable:
   CME_EventSignal_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady___TypedEmitInfo (slot
   0) */

undefined4 * __thiscall
CME_EventSignal_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00433190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_JVCookedSpecialWordsType_U.c ========== */

/*
 * SGW.exe - CME_EventSignal_JVCookedSpecialWordsType_U (2 functions)
 */

/* Also in vtable:
   CME_EventSignal_JVCookedSpecialWordsType_U__Event_Cache_ElementError___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_JVCookedSpecialWordsType_U__Event_Cache_ElementError___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_00422ed0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable:
   CME_EventSignal_JVCookedSpecialWordsType_U__Event_Cache_ElementReady___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_JVCookedSpecialWordsType_U__Event_Cache_ElementReady___TypedEmitInfo__vfunc_0
          (void *this,byte param_1)

{
  FUN_004337f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_SGWUIManager_UEvent_UI_StateChange.c ========== */

/*
 * SGW.exe - CME_EventSignal_SGWUIManager_UEvent_UI_StateChange (2 functions)
 */

/* Also in vtable: CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005708d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___CallbackImpl at 0183fa40
   Also in vtable:
   CME_EventSignal_ZU54_PAX_4_AEXPAUEvent_UI_StateChange_P84_VSGWUIManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___CallbackImpl__vfunc_2(void)

{
  return &SGWUIManager::Event_UI_StateChange::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_UEvent_Content_StreamingComplete.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Content_StreamingComplete (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_Content_StreamingComplete___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_Content_StreamingComplete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00554f00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Editor_BeginPIE.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Editor_BeginPIE (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_Editor_BeginPIE___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00be5470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Editor_ChunkManagerLoaded.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Editor_ChunkManagerLoaded (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_Editor_ChunkManagerLoaded___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0111fa70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated (3 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated___CallbackImpl at 017fa768
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JUCookedKismetEventSetData_U__Event_Cache_ElementError_JUCookedKismetEventSetData_U__Event_Cache_ElementReady_Detail_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ZipStorage_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady_Detail_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ZipStorage_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVMission_U__Event_Cache_ElementError_JVMission_U__Event_Cache_ElementReady_Detail_CME_VMission_V__RefCountedObj__02JVMission_V__ZipStorage_CME_VMission_V__RefCountedObj__02JVMission_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVDBInvItem_U__Event_Cache_ElementError_JVDBInvItem_U__Event_Cache_ElementReady_Detail_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ZipStorage_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVDBInvContainer_U__Event_Cache_ElementError_JVDBInvContainer_U__Event_Cache_ElementReady_Detail_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ZipStorage_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVDialog_U__Event_Cache_ElementError_JVDialog_U__Event_Cache_ElementReady_Detail_CME_VDialog_V__RefCountedObj__04JVDialog_V__ZipStorage_CME_VDialog_V__RefCountedObj__04JVDialog_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVInteractionSet_U__Event_Cache_ElementError_JVInteractionSet_U__Event_Cache_ElementReady_Detail_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ZipStorage_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVAbility_U__Event_Cache_ElementError_JVAbility_U__Event_Cache_ElementReady_Detail_CME_VAbility_V__RefCountedObj__01JVAbility_V__ZipStorage_CME_VAbility_V__RefCountedObj__01JVAbility_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVDBEffect_U__Event_Cache_ElementError_JVDBEffect_U__Event_Cache_ElementReady_Detail_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ZipStorage_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JUCookedCharCreationData_U__Event_Cache_ElementError_JUCookedCharCreationData_U__Event_Cache_ElementReady_Detail_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ZipStorage_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVDBGateInfo_U__Event_Cache_ElementError_JVDBGateInfo_U__Event_Cache_ElementReady_Detail_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ZipStorage_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_SGW_JVWorldInfo_U__Event_Cache_ElementError_SGW_JVWorldInfo_U__Event_Cache_ElementReady_Detail_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ZipStorage_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVCookedErrorTextType_U__Event_Cache_ElementError_JVCookedErrorTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ZipStorage_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVCookedTextType_U__Event_Cache_ElementError_JVCookedTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ZipStorage_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_SGW_JVBlueprint_U__Event_Cache_ElementError_SGW_JVBlueprint_U__Event_Cache_ElementReady_Detail_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ZipStorage_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_SGW_JVAppliedScience_U__Event_Cache_ElementError_SGW_JVAppliedScience_U__Event_Cache_ElementReady_Detail_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ZipStorage_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_SGW_JVDiscipline_U__Event_Cache_ElementError_SGW_JVDiscipline_U__Event_Cache_ElementReady_Detail_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ZipStorage_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_SGW_JVRacialParadigm_U__Event_Cache_ElementError_SGW_JVRacialParadigm_U__Event_Cache_ElementReady_Detail_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ZipStorage_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVCookedSpecialWordsType_U__Event_Cache_ElementError_JVCookedSpecialWordsType_U__Event_Cache_ElementReady_Detail_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ZipStorage_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_SGW_JVInteractionData_U__Event_Cache_ElementError_SGW_JVInteractionData_U__Event_Cache_ElementReady_Detail_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ZipStorage_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Entity_ProxyPlayerBaseCreated_P845_Detail_JVBehaviorEventData_U__Event_Cache_ElementError_JVBehaviorEventData_U__Event_Cache_ElementReady_Detail_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ZipStorage_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated___CallbackImpl__vfunc_2(void)

{
  return &Event_Entity_ProxyPlayerBaseCreated::RTTI_Type_Descriptor;
}


/* [VTable] CME_EventSignal_UEvent_Net_Connected___CallbackImpl virtual function #4
   VTable: vtable_CME_EventSignal_UEvent_Net_Connected___CallbackImpl at 017fa74c
   Also in vtable: CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Net_Disconnected___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_NetIn_onVersionInfo___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Net_ProxyData___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_NetIn_onCookedDataError___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_Option_MasterVolume___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_Option_MusicVolume___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Sys_FrameStart___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_Action_MouseLook___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_Action_MouseClick___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_Option_Rendering___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Input_KeyRelease___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Input_KeyPress___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_LoadScreen_TaskComplete___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_UI_SlashCommand___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_VEvent_UI_BindableAction___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Render_Reset___CallbackImpl (slot 4)
   Also in vtable: CME_EventSignal_UEvent_Sys_PreRender___CallbackImpl (slot 4) */

bool __thiscall
CME_EventSignal_UEvent_Net_Connected___CallbackImpl__vfunc_4(void *this,int *param_1)

{
  bool bVar1;
  type_info *ptVar2;
  type_info *this_00;
  
  ptVar2 = (type_info *)(**(code **)(*param_1 + 0xc))();
  this_00 = (type_info *)(**(code **)(*(int *)this + 0xc))();
  bVar1 = type_info::operator==(this_00,ptVar2);
  return bVar1;
}


/* Also in vtable: CME_EventSignal_UEvent_Net_Connected___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Entity_ProxyPlayerBaseCreated___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Net_Disconnected___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_NetIn_onVersionInfo___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Net_ProxyData___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_NetIn_onCookedDataError___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_Option_MasterVolume___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_Option_MusicVolume___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Sys_FrameStart___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_SGWUIManager_UEvent_UI_StateChange___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_Action_MouseLook___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_Action_MouseClick___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_Option_Rendering___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Input_KeyRelease___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Input_KeyPress___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_LoadScreen_TaskComplete___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_UI_SlashCommand___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_VEvent_UI_BindableAction___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Render_Reset___CallbackImpl (slot 0)
   Also in vtable: CME_EventSignal_UEvent_Sys_PreRender___CallbackImpl (slot 0) */

undefined4 * __thiscall
CME_EventSignal__03_U__BasicEvent_Entity_PropertyUpdate___CallbackImpl__vfunc_0
          (void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_016f53c8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::EventSignal::Callback::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== CME_EventSignal_UEvent_Entity_StateFieldChanged.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Entity_StateFieldChanged (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_Entity_StateFieldChanged___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e02f00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Input_KeyPress.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Input_KeyPress (1 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Input_KeyPress___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Input_KeyPress___CallbackImpl at 018406bc
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPAUEvent_Input_KeyPress_P84_VBindableActionManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Input_KeyPress___CallbackImpl__vfunc_2(void)

{
  return &Event_Input_KeyPress::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_UEvent_Input_KeyRelease.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Input_KeyRelease (1 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Input_KeyRelease___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Input_KeyRelease___CallbackImpl at 018406a0
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPAUEvent_Input_KeyRelease_P84_VBindableActionManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Input_KeyRelease___CallbackImpl__vfunc_2(void)

{
  return &Event_Input_KeyRelease::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_UEvent_LoadScreen_TaskComplete.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_LoadScreen_TaskComplete (2 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_LoadScreen_TaskComplete___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_LoadScreen_TaskComplete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00554fd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CME_EventSignal_UEvent_LoadScreen_TaskComplete___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_LoadScreen_TaskComplete___CallbackImpl at 018ebb34
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPBUEvent_LoadScreen_TaskComplete_P84_VUSGWLoadingScreen_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_LoadScreen_TaskComplete___CallbackImpl__vfunc_2(void)

{
  return &Event_LoadScreen_TaskComplete::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_UEvent_MapThumbnail_Deleted.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_MapThumbnail_Deleted (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_MapThumbnail_Deleted___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_01035ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Net_Connected.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Net_Connected (2 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Net_Connected___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Net_Connected___CallbackImpl at 017fa74c
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JUCookedKismetEventSetData_U__Event_Cache_ElementError_JUCookedKismetEventSetData_U__Event_Cache_ElementReady_Detail_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ZipStorage_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady_Detail_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ZipStorage_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVMission_U__Event_Cache_ElementError_JVMission_U__Event_Cache_ElementReady_Detail_CME_VMission_V__RefCountedObj__02JVMission_V__ZipStorage_CME_VMission_V__RefCountedObj__02JVMission_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVDBInvItem_U__Event_Cache_ElementError_JVDBInvItem_U__Event_Cache_ElementReady_Detail_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ZipStorage_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVDBInvContainer_U__Event_Cache_ElementError_JVDBInvContainer_U__Event_Cache_ElementReady_Detail_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ZipStorage_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVDialog_U__Event_Cache_ElementError_JVDialog_U__Event_Cache_ElementReady_Detail_CME_VDialog_V__RefCountedObj__04JVDialog_V__ZipStorage_CME_VDialog_V__RefCountedObj__04JVDialog_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVInteractionSet_U__Event_Cache_ElementError_JVInteractionSet_U__Event_Cache_ElementReady_Detail_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ZipStorage_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVAbility_U__Event_Cache_ElementError_JVAbility_U__Event_Cache_ElementReady_Detail_CME_VAbility_V__RefCountedObj__01JVAbility_V__ZipStorage_CME_VAbility_V__RefCountedObj__01JVAbility_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVDBEffect_U__Event_Cache_ElementError_JVDBEffect_U__Event_Cache_ElementReady_Detail_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ZipStorage_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JUCookedCharCreationData_U__Event_Cache_ElementError_JUCookedCharCreationData_U__Event_Cache_ElementReady_Detail_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ZipStorage_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVDBGateInfo_U__Event_Cache_ElementError_JVDBGateInfo_U__Event_Cache_ElementReady_Detail_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ZipStorage_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_SGW_JVWorldInfo_U__Event_Cache_ElementError_SGW_JVWorldInfo_U__Event_Cache_ElementReady_Detail_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ZipStorage_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVCookedErrorTextType_U__Event_Cache_ElementError_JVCookedErrorTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ZipStorage_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVCookedTextType_U__Event_Cache_ElementError_JVCookedTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ZipStorage_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_SGW_JVBlueprint_U__Event_Cache_ElementError_SGW_JVBlueprint_U__Event_Cache_ElementReady_Detail_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ZipStorage_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_SGW_JVAppliedScience_U__Event_Cache_ElementError_SGW_JVAppliedScience_U__Event_Cache_ElementReady_Detail_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ZipStorage_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_SGW_JVDiscipline_U__Event_Cache_ElementError_SGW_JVDiscipline_U__Event_Cache_ElementReady_Detail_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ZipStorage_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_SGW_JVRacialParadigm_U__Event_Cache_ElementError_SGW_JVRacialParadigm_U__Event_Cache_ElementReady_Detail_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ZipStorage_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVCookedSpecialWordsType_U__Event_Cache_ElementError_JVCookedSpecialWordsType_U__Event_Cache_ElementReady_Detail_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ZipStorage_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_SGW_JVInteractionData_U__Event_Cache_ElementError_SGW_JVInteractionData_U__Event_Cache_ElementReady_Detail_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ZipStorage_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Connected_P845_Detail_JVBehaviorEventData_U__Event_Cache_ElementError_JVBehaviorEventData_U__Event_Cache_ElementReady_Detail_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ZipStorage_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Net_Connected___CallbackImpl__vfunc_2(void)

{
  return &Event_Net_Connected::RTTI_Type_Descriptor;
}


undefined4 * __thiscall
CME_EventSignal_UEvent_Net_Connected___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c70030(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Net_Disconnected.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Net_Disconnected (2 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Net_Disconnected___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Net_Disconnected___CallbackImpl at 017fa784
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JUCookedKismetEventSetData_U__Event_Cache_ElementError_JUCookedKismetEventSetData_U__Event_Cache_ElementReady_Detail_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ZipStorage_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady_Detail_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ZipStorage_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVMission_U__Event_Cache_ElementError_JVMission_U__Event_Cache_ElementReady_Detail_CME_VMission_V__RefCountedObj__02JVMission_V__ZipStorage_CME_VMission_V__RefCountedObj__02JVMission_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVDBInvItem_U__Event_Cache_ElementError_JVDBInvItem_U__Event_Cache_ElementReady_Detail_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ZipStorage_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVDBInvContainer_U__Event_Cache_ElementError_JVDBInvContainer_U__Event_Cache_ElementReady_Detail_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ZipStorage_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVDialog_U__Event_Cache_ElementError_JVDialog_U__Event_Cache_ElementReady_Detail_CME_VDialog_V__RefCountedObj__04JVDialog_V__ZipStorage_CME_VDialog_V__RefCountedObj__04JVDialog_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVInteractionSet_U__Event_Cache_ElementError_JVInteractionSet_U__Event_Cache_ElementReady_Detail_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ZipStorage_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVAbility_U__Event_Cache_ElementError_JVAbility_U__Event_Cache_ElementReady_Detail_CME_VAbility_V__RefCountedObj__01JVAbility_V__ZipStorage_CME_VAbility_V__RefCountedObj__01JVAbility_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVDBEffect_U__Event_Cache_ElementError_JVDBEffect_U__Event_Cache_ElementReady_Detail_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ZipStorage_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JUCookedCharCreationData_U__Event_Cache_ElementError_JUCookedCharCreationData_U__Event_Cache_ElementReady_Detail_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ZipStorage_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVDBGateInfo_U__Event_Cache_ElementError_JVDBGateInfo_U__Event_Cache_ElementReady_Detail_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ZipStorage_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_SGW_JVWorldInfo_U__Event_Cache_ElementError_SGW_JVWorldInfo_U__Event_Cache_ElementReady_Detail_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ZipStorage_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVCookedErrorTextType_U__Event_Cache_ElementError_JVCookedErrorTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ZipStorage_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVCookedTextType_U__Event_Cache_ElementError_JVCookedTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ZipStorage_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_SGW_JVBlueprint_U__Event_Cache_ElementError_SGW_JVBlueprint_U__Event_Cache_ElementReady_Detail_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ZipStorage_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_SGW_JVAppliedScience_U__Event_Cache_ElementError_SGW_JVAppliedScience_U__Event_Cache_ElementReady_Detail_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ZipStorage_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_SGW_JVDiscipline_U__Event_Cache_ElementError_SGW_JVDiscipline_U__Event_Cache_ElementReady_Detail_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ZipStorage_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_SGW_JVRacialParadigm_U__Event_Cache_ElementError_SGW_JVRacialParadigm_U__Event_Cache_ElementReady_Detail_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ZipStorage_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVCookedSpecialWordsType_U__Event_Cache_ElementError_JVCookedSpecialWordsType_U__Event_Cache_ElementReady_Detail_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ZipStorage_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_SGW_JVInteractionData_U__Event_Cache_ElementError_SGW_JVInteractionData_U__Event_Cache_ElementReady_Detail_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ZipStorage_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPBUEvent_Net_Disconnected_P845_Detail_JVBehaviorEventData_U__Event_Cache_ElementError_JVBehaviorEventData_U__Event_Cache_ElementReady_Detail_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ZipStorage_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Net_Disconnected___CallbackImpl__vfunc_2(void)

{
  return &Event_Net_Disconnected::RTTI_Type_Descriptor;
}


undefined4 * __thiscall
CME_EventSignal_UEvent_Net_Disconnected___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c70100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Net_ProxyData.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Net_ProxyData (2 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Net_ProxyData___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Net_ProxyData___CallbackImpl at 017fa7bc
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JUCookedKismetEventSetData_U__Event_Cache_ElementError_JUCookedKismetEventSetData_U__Event_Cache_ElementReady_Detail_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ZipStorage_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady_Detail_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ZipStorage_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVMission_U__Event_Cache_ElementError_JVMission_U__Event_Cache_ElementReady_Detail_CME_VMission_V__RefCountedObj__02JVMission_V__ZipStorage_CME_VMission_V__RefCountedObj__02JVMission_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVDBInvItem_U__Event_Cache_ElementError_JVDBInvItem_U__Event_Cache_ElementReady_Detail_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ZipStorage_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVDBInvContainer_U__Event_Cache_ElementError_JVDBInvContainer_U__Event_Cache_ElementReady_Detail_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ZipStorage_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVDialog_U__Event_Cache_ElementError_JVDialog_U__Event_Cache_ElementReady_Detail_CME_VDialog_V__RefCountedObj__04JVDialog_V__ZipStorage_CME_VDialog_V__RefCountedObj__04JVDialog_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVInteractionSet_U__Event_Cache_ElementError_JVInteractionSet_U__Event_Cache_ElementReady_Detail_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ZipStorage_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVAbility_U__Event_Cache_ElementError_JVAbility_U__Event_Cache_ElementReady_Detail_CME_VAbility_V__RefCountedObj__01JVAbility_V__ZipStorage_CME_VAbility_V__RefCountedObj__01JVAbility_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVDBEffect_U__Event_Cache_ElementError_JVDBEffect_U__Event_Cache_ElementReady_Detail_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ZipStorage_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JUCookedCharCreationData_U__Event_Cache_ElementError_JUCookedCharCreationData_U__Event_Cache_ElementReady_Detail_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ZipStorage_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVDBGateInfo_U__Event_Cache_ElementError_JVDBGateInfo_U__Event_Cache_ElementReady_Detail_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ZipStorage_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_SGW_JVWorldInfo_U__Event_Cache_ElementError_SGW_JVWorldInfo_U__Event_Cache_ElementReady_Detail_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ZipStorage_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVCookedErrorTextType_U__Event_Cache_ElementError_JVCookedErrorTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ZipStorage_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVCookedTextType_U__Event_Cache_ElementError_JVCookedTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ZipStorage_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_SGW_JVBlueprint_U__Event_Cache_ElementError_SGW_JVBlueprint_U__Event_Cache_ElementReady_Detail_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ZipStorage_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_SGW_JVAppliedScience_U__Event_Cache_ElementError_SGW_JVAppliedScience_U__Event_Cache_ElementReady_Detail_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ZipStorage_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_SGW_JVDiscipline_U__Event_Cache_ElementError_SGW_JVDiscipline_U__Event_Cache_ElementReady_Detail_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ZipStorage_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_SGW_JVRacialParadigm_U__Event_Cache_ElementError_SGW_JVRacialParadigm_U__Event_Cache_ElementReady_Detail_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ZipStorage_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVCookedSpecialWordsType_U__Event_Cache_ElementError_JVCookedSpecialWordsType_U__Event_Cache_ElementReady_Detail_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ZipStorage_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_SGW_JVInteractionData_U__Event_Cache_ElementError_SGW_JVInteractionData_U__Event_Cache_ElementReady_Detail_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ZipStorage_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU6_PAX_AEXPAUEvent_Net_ProxyData_P845_Detail_JVBehaviorEventData_U__Event_Cache_ElementError_JVBehaviorEventData_U__Event_Cache_ElementReady_Detail_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ZipStorage_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Net_ProxyData___CallbackImpl__vfunc_2(void)

{
  return &Event_Net_ProxyData::RTTI_Type_Descriptor;
}


undefined4 * __thiscall
CME_EventSignal_UEvent_Net_ProxyData___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dd4220(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Options_Applied_Changes.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Options_Applied_Changes (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_Options_Applied_Changes___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_Options_Applied_Changes___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00575a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Options_KeyBindChanges.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Options_KeyBindChanges (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_Options_KeyBindChanges___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_Options_KeyBindChanges___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0057d260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Options_Loaded.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Options_Loaded (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_Options_Loaded___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_Options_Loaded___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00575b50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Player_GroundTargetingEnd.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Player_GroundTargetingEnd (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_Player_GroundTargetingEnd___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df5340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Player_TargetChange.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Player_TargetChange (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_Player_TargetChange___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df4ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Render_Reset.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Render_Reset (2 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Render_Reset___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Render_Reset___CallbackImpl at 0192192c
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPBUEvent_Render_Reset_P84_VUSGWTextComponent_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Render_Reset___CallbackImpl__vfunc_2(void)

{
  return &Event_Render_Reset::RTTI_Type_Descriptor;
}


undefined4 * __thiscall
CME_EventSignal_UEvent_Render_Reset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ec5a00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_Sys_FrameStart.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Sys_FrameStart (1 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Sys_FrameStart___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Sys_FrameStart___CallbackImpl at 0183f220
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPAUEvent_Sys_FrameStart_P84_VSGWExternalWindowManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPAUEvent_Sys_FrameStart_P84_VSGWUIManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPBUEvent_Sys_FrameStart_P84_VUSGWLoadingScreen_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPAUEvent_Sys_FrameStart_P84_VBindableAction_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Sys_FrameStart___CallbackImpl__vfunc_2(void)

{
  return &Event_Sys_FrameStart::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_UEvent_Sys_PreRender.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_Sys_PreRender (1 functions)
 */

/* [VTable] CME_EventSignal_UEvent_Sys_PreRender___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_UEvent_Sys_PreRender___CallbackImpl at 01921948
   Also in vtable:
   CME_EventSignal_ZU5_PAX_AEXPBUEvent_Sys_PreRender_P84_VUSGWTextComponent_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_UEvent_Sys_PreRender___CallbackImpl__vfunc_2(void)

{
  return &Event_Sys_PreRender::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_UEvent_UI_AbilitySetUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_AbilitySetUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_AbilitySetUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d2b2b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_AbilityUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_AbilityUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_AbilityUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d2b1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ActionUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ActionUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ActionUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e3f970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_AutoCycle.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_AutoCycle (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_AutoCycle___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e030c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_BMError.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_BMError (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_BMError___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5c0f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_BMOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_BMOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_BMOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5b2a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_BMViewReady.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_BMViewReady (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_BMViewReady___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5b3e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_BlueprintUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_BlueprintUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_BlueprintUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e455c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CharacterCreateFailed.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CharacterCreateFailed (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CharacterCreateFailed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e77b40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CharacterListUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CharacterListUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CharacterListUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e733e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CharacterSelectionUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CharacterSelectionUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CharacterSelectionUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e73310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommChannelJoined.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommChannelJoined (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommChannelJoined___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cfa7e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommChannelLeft.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommChannelLeft (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommChannelLeft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cfa870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommMessageReceived.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommMessageReceived (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommMessageReceived___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cfa6c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommTellSent.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommTellSent (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommTellSent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cfa900(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommandCreate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommandCreate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommandCreate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb2f60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommandInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommandInvite (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommandInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb34f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommandUpdateCash.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommandUpdateCash (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommandUpdateCash___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb2e20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CommandVaultVisibility.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CommandVaultVisibility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CommandVaultVisibility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e21820(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ContactListAddMembers.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ContactListAddMembers (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ContactListAddMembers___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5fc20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ContactListDelete.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ContactListDelete (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ContactListDelete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5fb50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ContactListMemberEvent.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ContactListMemberEvent (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ContactListMemberEvent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e62f10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ContactListRemoveMembers.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ContactListRemoveMembers (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ContactListRemoveMembers___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5fcf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ContactListUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ContactListUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ContactListUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5fa80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CraftInductionStart.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CraftInductionStart (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CraftInductionStart___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e45760(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CraftingAllowedUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CraftingAllowedUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CraftingAllowedUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e45350(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CraftingRespecPrompt.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CraftingRespecPrompt (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CraftingRespecPrompt___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e45690(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CreationBodyUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CreationBodyUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CreationBodyUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d345f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_CreationVisualsUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_CreationVisualsUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_CreationVisualsUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d346c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_DHDVisibility.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_DHDVisibility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_DHDVisibility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e2faf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_DialogAvailable.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_DialogAvailable (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_DialogAvailable___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d26b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_DialogDisplay.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_DialogDisplay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_DialogDisplay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d26be0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_DialogRemoved.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_DialogRemoved (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_DialogRemoved___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d26cb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_DuelTimerStart.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_DuelTimerStart (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_DuelTimerStart___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df5830(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_EntityReload.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_EntityReload (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_EntityReload___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e03260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_EntityReloadDeployment.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_EntityReloadDeployment (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_EntityReloadDeployment___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e03330(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ExperienceUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ExperienceUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ExperienceUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df5410(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_Initialized.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_Initialized (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_UI_Initialized___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_Initialized___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005709a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_InventoryClear.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_InventoryClear (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_InventoryClear___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e21340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_InventoryHideSlot.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_InventoryHideSlot (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_InventoryHideSlot___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e21270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_InventoryUpdateCash.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_InventoryUpdateCash (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_InventoryUpdateCash___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e215b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_InventoryUpdateSlot.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_InventoryUpdateSlot (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_InventoryUpdateSlot___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e211a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_LevelChanged.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_LevelChanged (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_LevelChanged___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e02ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_LoginFailed.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_LoginFailed (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_LoginFailed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dfdfc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_LoginSuccess.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_LoginSuccess (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_LoginSuccess___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df5190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_LootDisplay.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_LootDisplay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_LootDisplay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e24790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailCloseNewMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailCloseNewMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailCloseNewMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e170a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailDisplayMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailDisplayMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailDisplayMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e173e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailItemAttachmentUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailItemAttachmentUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailItemAttachmentUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e17240(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailMessageRemoved.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailMessageRemoved (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailMessageRemoved___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e17580(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailReactivateNewMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailReactivateNewMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailReactivateNewMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e17170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailUpdateMailbox.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailUpdateMailbox (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailUpdateMailbox___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e17310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MailUpdateMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MailUpdateMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MailUpdateMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e174b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameCallDisplay.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameCallDisplay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameCallDisplay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e3c8c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameCallHide.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameCallHide (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameCallHide___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e381a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameCallResult.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameCallResult (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameCallResult___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e38410(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameContactCount.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameContactCount (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameContactCount___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e384e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameName.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameName (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameName___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e3c7a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameObserver.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameObserver (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameObserver___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e3c9e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameProgressBar.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameProgressBar (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameProgressBar___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e38270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameSecondsLeft.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameSecondsLeft (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameSecondsLeft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e38340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameSpectateList.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameSpectateList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameSpectateList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e37b80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameText.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameText (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_UI_MinigameText___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameText___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0056a1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MinigameVisibility.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MinigameVisibility (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_UI_MinigameVisibility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MinigameVisibility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005696c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MissionLogUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MissionLogUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MissionLogUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1b8e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MissionReceived.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MissionReceived (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MissionReceived___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1b9b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MissionRemoved.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MissionRemoved (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MissionRemoved___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1ba80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MissionRewards.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MissionRewards (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MissionRewards___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1bb50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MissionTrackerUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MissionTrackerUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MissionTrackerUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1b740(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_MissionUpdated.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_MissionUpdated (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_MissionUpdated___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1b810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_NetDisconnect.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_NetDisconnect (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_NetDisconnect___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df50c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_Options_Change_Applied.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_Options_Change_Applied (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_Options_Change_Applied___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df54e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ParadigmLevelUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ParadigmLevelUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ParadigmLevelUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e45420(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_PetStanceChange.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_PetStanceChange (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_PetStanceChange___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d3ad00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_PetUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_PetUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_PetUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d3add0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_RemoveMinigameHelper.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_RemoveMinigameHelper (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_RemoveMinigameHelper___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e38060(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_ServerSelectionMade.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_ServerSelectionMade (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_ServerSelectionMade___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dfe050(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_SpaceQueueReady.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_SpaceQueueReady (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_SpaceQueueReady___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df5620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_SpaceQueued.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_SpaceQueued (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_SpaceQueued___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00df56f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_SplashMessageReceived.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_SplashMessageReceived (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_SplashMessageReceived___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cfa750(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_SquadChangedAll.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_SquadChangedAll (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_SquadChangedAll___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5e3d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_SquadInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_SquadInvite (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_SquadInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5f3a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_StartMinigameDialog.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_StartMinigameDialog (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_StartMinigameDialog___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e3c830(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_StartMinigameDialogClose.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_StartMinigameDialogClose (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_StartMinigameDialogClose___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e37ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_StrikeTeamUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_StrikeTeamUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_StrikeTeamUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5e510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TeamCreate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TeamCreate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TeamCreate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb40a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TeamInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TeamInvite (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TeamInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb4760(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TeamUpdateCash.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TeamUpdateCash (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TeamUpdateCash___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb3f50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TeamVaultVisibility.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TeamVaultVisibility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TeamVaultVisibility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e21750(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TradeRemoteUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TradeRemoteUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TradeRemoteUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e26790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TrainerOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TrainerOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TrainerOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e19700(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_TrainerUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_TrainerUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_TrainerUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e197d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UnitCombat.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UnitCombat (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UnitCombat___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb1050(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UnitEffectsUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UnitEffectsUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UnitEffectsUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e09790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UnitStealth.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UnitStealth (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UnitStealth___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e03190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UpdateCommandMember.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UpdateCommandMember (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UpdateCommandMember___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb3460(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UpdateOrganization.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UpdateOrganization (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UpdateOrganization___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e53520(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UpdateOrganizationMOTD.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UpdateOrganizationMOTD (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UpdateOrganizationMOTD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e535f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_UpdateTeamMember.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_UpdateTeamMember (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_UpdateTeamMember___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00eb46d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_VaultVisibility.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_VaultVisibility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_VaultVisibility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e21680(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_VendorClose.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_VendorClose (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_VendorClose___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e11b20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_VendorOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_VendorOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_VendorOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e11a50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_VendorUpdateSlot.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_VendorUpdateSlot (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_VendorUpdateSlot___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e11bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_WorldChanged.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_WorldChanged (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_WorldChanged___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c72a60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_onDuelChallenge.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_onDuelChallenge (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_onDuelChallenge___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dfe720(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_UI_onRingTransporterList.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_UI_onRingTransporterList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_UEvent_UI_onRingTransporterList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dfe690(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_World_DialStargateAddress.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_World_DialStargateAddress (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_World_DialStargateAddress___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_World_DialStargateAddress___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00569800(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_UEvent_World_StargateEvent.c ========== */

/*
 * SGW.exe - CME_EventSignal_UEvent_World_StargateEvent (1 functions)
 */

/* Also in vtable: CME_EventSignal_UEvent_World_StargateEvent___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_UEvent_World_StargateEvent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005698d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_CancelTarget.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_CancelTarget (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_CancelTarget___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_CancelTarget___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00584df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_Crouch.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_Crouch (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_Crouch___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_Crouch___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005843f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_CycleTargetBackward.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_CycleTargetBackward (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_CycleTargetBackward___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_CycleTargetBackward___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005852f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_CycleTargetForward.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_CycleTargetForward (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_CycleTargetForward___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_CycleTargetForward___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00585070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_Jump.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_Jump (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_Jump___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_Jump___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00584170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_MouseClick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_MouseClick (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_Action_MouseClick___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_Action_MouseClick___CallbackImpl at 0183fa78
   Also in vtable:
   CME_EventSignal_ZVEvent_Action_MouseClick_PAX_3_AEXPAVGenericEvent_P84_VSGWUIManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_Action_MouseClick___CallbackImpl__vfunc_2(void)

{
  return &Event_Action_MouseClick::RTTI_Type_Descriptor;
}


/* Also in vtable: CME_EventSignal_VEvent_Action_MouseClick___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_MouseClick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00582d60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_MouseLook.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_MouseLook (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_Action_MouseLook___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_Action_MouseLook___CallbackImpl at 0183fa5c
   Also in vtable:
   CME_EventSignal_ZVEvent_Action_MouseLook_PAX_3_AEXPAVGenericEvent_P84_VSGWUIManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_Action_MouseLook___CallbackImpl__vfunc_2(void)

{
  return &Event_Action_MouseLook::RTTI_Type_Descriptor;
}


/* Also in vtable: CME_EventSignal_VEvent_Action_MouseLook___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_MouseLook___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00582ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_MoveBack.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_MoveBack (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_MoveBack___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_MoveBack___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005834f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_MoveForward.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_MoveForward (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_MoveForward___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_MoveForward___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00583270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_StepLeft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_StepLeft (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_StepLeft___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_StepLeft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00583770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_StepRight.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_StepRight (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_StepRight___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_StepRight___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005839f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPartyPet1.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPartyPet1 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPartyPet1___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPartyPet1___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00586e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPartyPet2.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPartyPet2 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPartyPet2___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPartyPet2___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005870f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPartyPet3.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPartyPet3 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPartyPet3___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPartyPet3___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00587370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPartyPet4.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPartyPet4 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPartyPet4___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPartyPet4___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005875f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPartyPet5.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPartyPet5 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPartyPet5___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPartyPet5___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00587870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPartyPet6.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPartyPet6 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPartyPet6___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPartyPet6___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00587af0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetPet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetPet (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetPet___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetPet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00586bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSelf.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSelf (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSelf___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSelf___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00585a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSquadMember1.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSquadMember1 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSquadMember1___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSquadMember1___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00585cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSquadMember2.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSquadMember2 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSquadMember2___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSquadMember2___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00585f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSquadMember3.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSquadMember3 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSquadMember3___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSquadMember3___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005861f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSquadMember4.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSquadMember4 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSquadMember4___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSquadMember4___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00586470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSquadMember5.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSquadMember5 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSquadMember5___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSquadMember5___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005866f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TargetSquadMember6.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TargetSquadMember6 (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TargetSquadMember6___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TargetSquadMember6___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00586970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_ToggleAutorun.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_ToggleAutorun (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_ToggleAutorun___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_ToggleAutorun___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00587d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TurnLeft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TurnLeft (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TurnLeft___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TurnLeft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00583c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_TurnRight.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_TurnRight (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_TurnRight___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_TurnRight___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00583ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_Walk.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_Walk (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_Walk___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_Walk___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00584670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_ZoomIn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_ZoomIn (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_ZoomIn___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_ZoomIn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005848f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Action_ZoomOut.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Action_ZoomOut (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Action_ZoomOut___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Action_ZoomOut___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00584b70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_Camera1stPerson.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_Camera1stPerson (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_Camera1stPerson___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_Camera1stPerson___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058b470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_Camera3rdPerson.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_Camera3rdPerson (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_Camera3rdPerson___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_Camera3rdPerson___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058b6f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_CameraDefault.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_CameraDefault (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_CameraDefault___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_CameraDefault___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058b1f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_CameraFixed.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_CameraFixed (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_CameraFixed___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_CameraFixed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058b970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_CameraFixedTracking.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_CameraFixedTracking (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_CameraFixedTracking___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_CameraFixedTracking___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058bbf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_CameraFree.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_CameraFree (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_CameraFree___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_CameraFree___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058be70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_Close.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_Close (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_Close___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_Close___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005893f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_Ghost.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_Ghost (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_Ghost___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_Ghost___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058c0f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_PIEScriptLoad.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_PIEScriptLoad (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_PIEScriptLoad___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_PIEScriptLoad___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058cff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_PIEScriptStart.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_PIEScriptStart (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_PIEScriptStart___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_PIEScriptStart___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058d270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_PIEScriptStep.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_PIEScriptStep (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_PIEScriptStep___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_PIEScriptStep___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058d770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_PIEScriptTogglePause.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_PIEScriptTogglePause (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_PIEScriptTogglePause___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_PIEScriptTogglePause___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058d4f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ScreenShot.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ScreenShot (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ScreenShot___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ScreenShot___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058acf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_SequenceBegin.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_SequenceBegin (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_SequenceBegin___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_SequenceBegin___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00589670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_SequenceEnd.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_SequenceEnd (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_SequenceEnd___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_SequenceEnd___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00589b70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_SequenceInterrupt.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_SequenceInterrupt (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_SequenceInterrupt___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_SequenceInterrupt___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005898f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_SetPIEScript1Active.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_SetPIEScript1Active (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_SetPIEScript1Active___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_SetPIEScript1Active___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058caf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_SetPIEScript2Active.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_SetPIEScript2Active (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_SetPIEScript2Active___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_SetPIEScript2Active___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058cd70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ShadowStats.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ShadowStats (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ShadowStats___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ShadowStats___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058af70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ShowFPS.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ShowFPS (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ShowFPS___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ShowFPS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058aa70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ShowPerformance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ShowPerformance (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ShowPerformance___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ShowPerformance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058a7f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_TestSequence.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_TestSequence (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_TestSequence___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_TestSequence___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00589170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ToggleCombat.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ToggleCombat (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ToggleCombat___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ToggleCombat___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058c870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_TogglePhysicsMode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_TogglePhysicsMode (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_TogglePhysicsMode___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_TogglePhysicsMode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00589df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_Use.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_Use (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_Use___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_Use___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058c5f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ViewLit.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ViewLit (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ViewLit___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ViewLit___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058a570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ViewUnlit.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ViewUnlit (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ViewUnlit___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ViewUnlit___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058a2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_ViewWireframe.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_ViewWireframe (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_ViewWireframe___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_ViewWireframe___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058a070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Editor_Walk.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Editor_Walk (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Editor_Walk___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Editor_Walk___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058c370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Entity_StatUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Entity_StatUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_Entity_StatUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dc7cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_AbilityTreeInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_AbilityTreeInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_AbilityTreeInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d802d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_AccountLoginSuccess.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_AccountLoginSuccess (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_AccountLoginSuccess___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d759b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_BMAuctionRemove.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_BMAuctionRemove (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_BMAuctionRemove___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d84720(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_BMAuctionUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_BMAuctionUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_BMAuctionUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d84480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_BMAuctions.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_BMAuctions (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_BMAuctions___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d841e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_BMError.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_BMError (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_BMError___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d849c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_BMOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_BMOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_BMOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d83f40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_BeingAppearance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_BeingAppearance (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_BeingAppearance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d81fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_CashChanged.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_CashChanged (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_CashChanged___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7e340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_CharacterCreateFailed.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_CharacterCreateFailed (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_CharacterCreateFailed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d78f40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_CharacterList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_CharacterList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_CharacterList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d78a00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_CharacterVisuals.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_CharacterVisuals (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_CharacterVisuals___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d78ca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_DialogDisplay.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_DialogDisplay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_DialogDisplay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d293c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_EffectUserData.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_EffectUserData (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_EffectUserData___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7f840(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_EntityFlags.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_EntityFlags (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_EntityFlags___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d784c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_EntityProperty.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_EntityProperty (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_EntityProperty___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d78760(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_EntityTint.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_EntityTint (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_EntityTint___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d78220(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_InitialInteraction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_InitialInteraction (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_InitialInteraction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7fae0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_InteractionType.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_InteractionType (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_InteractionType___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9c940(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_KnownAbilitiesUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_KnownAbilitiesUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_KnownAbilitiesUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d77500(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_LoginFailure.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_LoginFailure (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_LoginFailure___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d75eb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_LootDisplay.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_LootDisplay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_LootDisplay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d80570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MapInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MapInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MapInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d88620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MinigameCallAbort.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MinigameCallAbort (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MinigameCallAbort___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7c3c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MinigameCallDisplay.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MinigameCallDisplay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MinigameCallDisplay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7be80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MinigameCallResult.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MinigameCallResult (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MinigameCallResult___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7c120(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MissionOffer.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MissionOffer (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MissionOffer___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d817d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MissionRewards.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MissionRewards (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MissionRewards___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d81d10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_MissionSharedOffer.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_MissionSharedOffer (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_MissionSharedOffer___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d81a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_PetAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_PetAbilities (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_PetAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d777a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_PetStanceUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_PetStanceUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_PetStanceUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d77ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_PetStances.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_PetStances (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_PetStances___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d77a40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_PlayMovie.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_PlayMovie (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_PlayMovie___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8e220(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_RemoveMinigameHelper.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_RemoveMinigameHelper (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_RemoveMinigameHelper___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7bbe0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_ResetMapInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_ResetMapInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_ResetMapInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d888c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_ServerSelectSuccess.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_ServerSelectSuccess (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_ServerSelectSuccess___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d75c30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_ShowMinigameContact.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_ShowMinigameContact (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_ShowMinigameContact___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7c660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_ShowNavigation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_ShowNavigation (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_ShowNavigation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8fc60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_StargateTriggerFailed.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_StargateTriggerFailed (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_StargateTriggerFailed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d880e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_StopMovie.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_StopMovie (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_StopMovie___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8e4c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_TimerUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_TimerUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_TimerUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7f5a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_TradeResults.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_TradeResults (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_TradeResults___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d80ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_TradeState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_TradeState (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_TradeState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d80810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onActiveSlotUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onActiveSlotUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onActiveSlotUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7d620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onAlignmentUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onAlignmentUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onAlignmentUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d86be0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onArchetypeUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onArchetypeUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onArchetypeUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d88b60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onBeginAidWait.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onBeginAidWait (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onBeginAidWait___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d79480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onBeingNameIDUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onBeingNameIDUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onBeingNameIDUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d856e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onBeingNameUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onBeingNameUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onBeingNameUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d85440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onCharacterLoadFailed.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onCharacterLoadFailed (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onCharacterLoadFailed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d791e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onChatJoined.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onChatJoined (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onChatJoined___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d82500(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onChatLeft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onChatLeft (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onChatLeft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d82a40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onClientChallenge.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onClientChallenge (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onClientChallenge___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8d7a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onClientMapLoad.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onClientMapLoad (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onClientMapLoad___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d85ec0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onCommandVaultOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onCommandVaultOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onCommandVaultOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7eb20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onContactListDelete.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onContactListDelete (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onContactListDelete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8f1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onContactListEvent.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onContactListEvent (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onContactListEvent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8f9c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onContactListUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onContactListUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onContactListUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8ef40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onContainerInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onContainerInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onContainerInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7d380(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onCookedDataError.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onCookedDataError (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_NetIn_onCookedDataError___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_NetIn_onCookedDataError___CallbackImpl at 017fa7d8
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JUCookedKismetEventSetData_U__Event_Cache_ElementError_JUCookedKismetEventSetData_U__Event_Cache_ElementReady_Detail_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ZipStorage_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady_Detail_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ZipStorage_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVMission_U__Event_Cache_ElementError_JVMission_U__Event_Cache_ElementReady_Detail_CME_VMission_V__RefCountedObj__02JVMission_V__ZipStorage_CME_VMission_V__RefCountedObj__02JVMission_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVDBInvItem_U__Event_Cache_ElementError_JVDBInvItem_U__Event_Cache_ElementReady_Detail_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ZipStorage_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVDBInvContainer_U__Event_Cache_ElementError_JVDBInvContainer_U__Event_Cache_ElementReady_Detail_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ZipStorage_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVDialog_U__Event_Cache_ElementError_JVDialog_U__Event_Cache_ElementReady_Detail_CME_VDialog_V__RefCountedObj__04JVDialog_V__ZipStorage_CME_VDialog_V__RefCountedObj__04JVDialog_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVInteractionSet_U__Event_Cache_ElementError_JVInteractionSet_U__Event_Cache_ElementReady_Detail_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ZipStorage_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVAbility_U__Event_Cache_ElementError_JVAbility_U__Event_Cache_ElementReady_Detail_CME_VAbility_V__RefCountedObj__01JVAbility_V__ZipStorage_CME_VAbility_V__RefCountedObj__01JVAbility_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVDBEffect_U__Event_Cache_ElementError_JVDBEffect_U__Event_Cache_ElementReady_Detail_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ZipStorage_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JUCookedCharCreationData_U__Event_Cache_ElementError_JUCookedCharCreationData_U__Event_Cache_ElementReady_Detail_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ZipStorage_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVDBGateInfo_U__Event_Cache_ElementError_JVDBGateInfo_U__Event_Cache_ElementReady_Detail_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ZipStorage_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_SGW_JVWorldInfo_U__Event_Cache_ElementError_SGW_JVWorldInfo_U__Event_Cache_ElementReady_Detail_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ZipStorage_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVCookedErrorTextType_U__Event_Cache_ElementError_JVCookedErrorTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ZipStorage_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVCookedTextType_U__Event_Cache_ElementError_JVCookedTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ZipStorage_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_SGW_JVBlueprint_U__Event_Cache_ElementError_SGW_JVBlueprint_U__Event_Cache_ElementReady_Detail_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ZipStorage_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_SGW_JVAppliedScience_U__Event_Cache_ElementError_SGW_JVAppliedScience_U__Event_Cache_ElementReady_Detail_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ZipStorage_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_SGW_JVDiscipline_U__Event_Cache_ElementError_SGW_JVDiscipline_U__Event_Cache_ElementReady_Detail_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ZipStorage_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_SGW_JVRacialParadigm_U__Event_Cache_ElementError_SGW_JVRacialParadigm_U__Event_Cache_ElementReady_Detail_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ZipStorage_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVCookedSpecialWordsType_U__Event_Cache_ElementError_JVCookedSpecialWordsType_U__Event_Cache_ElementReady_Detail_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ZipStorage_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_SGW_JVInteractionData_U__Event_Cache_ElementError_SGW_JVInteractionData_U__Event_Cache_ElementReady_Detail_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ZipStorage_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPAVEvent_NetIn_onCookedDataError_P845_Detail_JVBehaviorEventData_U__Event_Cache_ElementError_JVBehaviorEventData_U__Event_Cache_ElementReady_Detail_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ZipStorage_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_NetIn_onCookedDataError___CallbackImpl__vfunc_2(void)

{
  return &Event_NetIn_onCookedDataError::RTTI_Type_Descriptor;
}


undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onCookedDataError___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d89340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDHDReply.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDHDReply (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDHDReply___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d82ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDisableShowPath.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDisableShowPath (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDisableShowPath___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9c6a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDisciplineRespec.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDisciplineRespec (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDisciplineRespec___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d83ca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDisplayDHD.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDisplayDHD (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDisplayDHD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7a440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDuelChallenge.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDuelChallenge (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDuelChallenge___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d89880(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDuelEntitiesClear.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDuelEntitiesClear (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDuelEntitiesClear___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8a060(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDuelEntitiesRemove.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDuelEntitiesRemove (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDuelEntitiesRemove___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d89dc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onDuelEntitiesSet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onDuelEntitiesSet (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onDuelEntitiesSet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d89b20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onEffectResults.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onEffectResults (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onEffectResults___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d77260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onEndAidWait.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onEndAidWait (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onEndAidWait___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d79720(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onEndMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onEndMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onEndMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7a980(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onEntityMove.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onEntityMove (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onEntityMove___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d85980(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onErrorCode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onErrorCode (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onErrorCode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d77f80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onExpUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onExpUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onExpUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d79f00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onExtraNameUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onExtraNameUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onExtraNameUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d79c60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onFactionUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onFactionUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onFactionUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d86e80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onLOSResult.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onLOSResult (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onLOSResult___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d82260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onLevelUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onLevelUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onLevelUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d84f00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onMailHeaderInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onMailHeaderInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onMailHeaderInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7c900(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onMailHeaderRemove.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onMailHeaderRemove (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onMailHeaderRemove___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7cba0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onMailRead.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onMailRead (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onMailRead___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7ce40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onMaxExpUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onMaxExpUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onMaxExpUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7a1a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onMeleeRangeUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onMeleeRangeUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onMeleeRangeUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d86160(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onMissionUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onMissionUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onMissionUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d80d50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onNickChanged.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onNickChanged (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onNickChanged___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d827a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onObjectiveUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onObjectiveUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onObjectiveUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d81290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onOrganizationInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onOrganizationInvite (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onOrganizationInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8a300(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onOrganizationJoined.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onOrganizationJoined (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onOrganizationJoined___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8a5a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onOrganizationLeft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onOrganizationLeft (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onOrganizationLeft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8a840(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onPlayerCommunication.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onPlayerCommunication (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onPlayerCommunication___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d767e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onPlayerDataLoaded.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onPlayerDataLoaded (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onPlayerDataLoaded___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d76540(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onRefreshItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onRefreshItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onRefreshItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7de00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onRemoteEntityCreate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onRemoteEntityCreate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onRemoteEntityCreate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8da40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onRemoteEntityMove.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onRemoteEntityMove (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onRemoteEntityMove___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8dce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onRemoteEntityRemove.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onRemoteEntityRemove (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onRemoteEntityRemove___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8df80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onRemoveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onRemoveItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onRemoveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7d8c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onRingTransporterList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onRingTransporterList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onRingTransporterList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d890a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSendMailResult.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSendMailResult (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSendMailResult___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7d0e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSequence.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSequence (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSequence___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d76fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSetTarget.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSetTarget (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSetTarget___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d895e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onShowPath.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onShowPath (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onShowPath___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9c400(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSpaceQueueReady.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSpaceQueueReady (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSpaceQueueReady___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9d3c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSpaceQueued.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSpaceQueued (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSpaceQueued___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9d660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSpectateList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSpectateList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSpectateList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7b160(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSquadLootType.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSquadLootType (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSquadLootType___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8cd10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStargatePassage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStargatePassage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStargatePassage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d88380(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStartMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStartMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStartMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7a6e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStartMinigameDialog.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStartMinigameDialog (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStartMinigameDialog___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7ac20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStatBaseUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStatBaseUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStatBaseUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d86940(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStatUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStatUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStatUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d866a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStateFieldUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStateFieldUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStateFieldUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d86400(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStepUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStepUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStepUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d80ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStoreClose.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStoreClose (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStoreClose___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7f300(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStoreOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStoreOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStoreOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7edc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStoreUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStoreUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStoreUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7f060(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onStrikeTeamUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onStrikeTeamUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onStrikeTeamUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8ea00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onSystemCommunication.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onSystemCommunication (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onSystemCommunication___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d76d20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onTargetUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onTargetUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onTargetUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d851a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onTaskUpdate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onTaskUpdate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onTaskUpdate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d81530(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onTeamVaultOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onTeamVaultOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onTeamVaultOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7e880(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onTellSent.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onTellSent (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onTellSent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d82f80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onTimeofDay.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onTimeofDay (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onTimeofDay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onTrainerOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onTrainerOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onTrainerOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d80030(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onUpdateDiscipline.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onUpdateDiscipline (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onUpdateDiscipline___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d83220(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onUpdateItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onUpdateItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onUpdateItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7db60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onUpdateKnownCrafts.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onUpdateKnownCrafts (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onUpdateKnownCrafts___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d83760(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onVaultOpen.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onVaultOpen (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onVaultOpen___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d7e5e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onVersionInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onVersionInfo (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_NetIn_onVersionInfo___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_NetIn_onVersionInfo___CallbackImpl at 017fa7a0
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JUCookedKismetEventSetData_U__Event_Cache_ElementError_JUCookedKismetEventSetData_U__Event_Cache_ElementReady_Detail_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ZipStorage_CME_UCookedKismetEventSetData_V__RefCountedObj__05JUCookedKismetEventSetData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVCookedKismetEventSequenceData_U__Event_Cache_ElementError_JVCookedKismetEventSequenceData_U__Event_Cache_ElementReady_Detail_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ZipStorage_CME_VCookedKismetEventSequenceData_V__RefCountedObj__00JVCookedKismetEventSequenceData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVMission_U__Event_Cache_ElementError_JVMission_U__Event_Cache_ElementReady_Detail_CME_VMission_V__RefCountedObj__02JVMission_V__ZipStorage_CME_VMission_V__RefCountedObj__02JVMission_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVDBInvItem_U__Event_Cache_ElementError_JVDBInvItem_U__Event_Cache_ElementReady_Detail_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ZipStorage_CME_VDBInvItem_V__RefCountedObj__03JVDBInvItem_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVDBInvContainer_U__Event_Cache_ElementError_JVDBInvContainer_U__Event_Cache_ElementReady_Detail_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ZipStorage_CME_VDBInvContainer_V__RefCountedObj_JVDBInvContainer__0O_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVDialog_U__Event_Cache_ElementError_JVDialog_U__Event_Cache_ElementReady_Detail_CME_VDialog_V__RefCountedObj__04JVDialog_V__ZipStorage_CME_VDialog_V__RefCountedObj__04JVDialog_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVInteractionSet_U__Event_Cache_ElementError_JVInteractionSet_U__Event_Cache_ElementReady_Detail_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ZipStorage_CME_VInteractionSet_V__RefCountedObj__07JVInteractionSet_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVAbility_U__Event_Cache_ElementError_JVAbility_U__Event_Cache_ElementReady_Detail_CME_VAbility_V__RefCountedObj__01JVAbility_V__ZipStorage_CME_VAbility_V__RefCountedObj__01JVAbility_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVDBEffect_U__Event_Cache_ElementError_JVDBEffect_U__Event_Cache_ElementReady_Detail_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ZipStorage_CME_VDBEffect_V__RefCountedObj__08JVDBEffect_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JUCookedCharCreationData_U__Event_Cache_ElementError_JUCookedCharCreationData_U__Event_Cache_ElementReady_Detail_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ZipStorage_CME_UCookedCharCreationData_V__RefCountedObj__06JUCookedCharCreationData_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVDBGateInfo_U__Event_Cache_ElementError_JVDBGateInfo_U__Event_Cache_ElementReady_Detail_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ZipStorage_CME_VDBGateInfo_V__RefCountedObj_JVDBGateInfo__0N_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_SGW_JVWorldInfo_U__Event_Cache_ElementError_SGW_JVWorldInfo_U__Event_Cache_ElementReady_Detail_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ZipStorage_CME_SGW_VWorldInfo_V__RefCountedObj_SGW_JVWorldInfo__0M_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVCookedErrorTextType_U__Event_Cache_ElementError_JVCookedErrorTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ZipStorage_CME_VCookedErrorTextType_V__RefCountedObj_JVCookedErrorTextType__0L_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVCookedTextType_U__Event_Cache_ElementError_JVCookedTextType_U__Event_Cache_ElementReady_Detail_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ZipStorage_CME_VCookedTextType_V__RefCountedObj__09JVCookedTextType_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_SGW_JVBlueprint_U__Event_Cache_ElementError_SGW_JVBlueprint_U__Event_Cache_ElementReady_Detail_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ZipStorage_CME_SGW_VBlueprint_V__RefCountedObj_SGW_JVBlueprint__0P_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_SGW_JVAppliedScience_U__Event_Cache_ElementError_SGW_JVAppliedScience_U__Event_Cache_ElementReady_Detail_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ZipStorage_CME_SGW_VAppliedScience_V__RefCountedObj_SGW_JVAppliedScience__0BA_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_SGW_JVDiscipline_U__Event_Cache_ElementError_SGW_JVDiscipline_U__Event_Cache_ElementReady_Detail_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ZipStorage_CME_SGW_VDiscipline_V__RefCountedObj_SGW_JVDiscipline__0BB_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_SGW_JVRacialParadigm_U__Event_Cache_ElementError_SGW_JVRacialParadigm_U__Event_Cache_ElementReady_Detail_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ZipStorage_CME_SGW_VRacialParadigm_V__RefCountedObj_SGW_JVRacialParadigm__0BC_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVCookedSpecialWordsType_U__Event_Cache_ElementError_JVCookedSpecialWordsType_U__Event_Cache_ElementReady_Detail_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ZipStorage_CME_VCookedSpecialWordsType_V__RefCountedObj_JVCookedSpecialWordsType__0BD_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_SGW_JVInteractionData_U__Event_Cache_ElementError_SGW_JVInteractionData_U__Event_Cache_ElementReady_Detail_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ZipStorage_CME_SGW_VInteractionData_V__RefCountedObj_SGW_JVInteractionData__0BE_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2)
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_NetIn_onVersionInfo_P845_Detail_JVBehaviorEventData_U__Event_Cache_ElementError_JVBehaviorEventData_U__Event_Cache_ElementReady_Detail_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ZipStorage_CME_VBehaviorEventData_V__RefCountedObj_JVBehaviorEventData__0BF_V__ServerSource_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_NetIn_onVersionInfo___CallbackImpl__vfunc_2(void)

{
  return &Event_NetIn_onVersionInfo::RTTI_Type_Descriptor;
}


undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onVersionInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d762a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_onVisible.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_onVisible (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_onVisible___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d85c20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_setupStargateInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_setupStargateInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_setupStargateInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d873c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_setupWorldParameters.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_setupWorldParameters (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_setupWorldParameters___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d87120(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetIn_updateStargateAddress.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetIn_updateStargateAddress (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetIn_updateStargateAddress___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d87660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_AbandonMission.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_AbandonMission (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_AbandonMission___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1b560(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_AbilityDebug.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_AbilityDebug (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_AbilityDebug___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90210(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_AddBehaviorEventSet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_AddBehaviorEventSet (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_AddBehaviorEventSet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Alloy.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Alloy (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Alloy___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e45110(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ArchiveMailMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ArchiveMailMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ArchiveMailMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d994c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_BMCancelAuction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_BMCancelAuction (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_BMCancelAuction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5c520(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_BMCreateAuction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_BMCreateAuction (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_BMCreateAuction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5c280(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_BMPlaceBid.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_BMPlaceBid (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_BMPlaceBid___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5c7c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_BMSearch.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_BMSearch (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_BMSearch___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5ca60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_BroadcastMinimapPing.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_BroadcastMinimapPing (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_BroadcastMinimapPing___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_BroadcastMinimapPing___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ad74b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_BuybackItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_BuybackItems (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_BuybackItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d98260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_CancelMovie.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_CancelMovie (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_CancelMovie___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9b980(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChangeCoverWeight.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChangeCoverWeight (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChangeCoverWeight___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93210(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChangeWeaponState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChangeWeaponState (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChangeWeaponState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9b1a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatBan.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatBan (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatBan___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf4440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatFriend.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatFriend (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatFriend___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatIgnore.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatIgnore (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatIgnore___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf4030(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatJoin.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatJoin (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatJoin___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf3ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatKick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatKick (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatKick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf42a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatLeave.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatLeave (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatLeave___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf3db0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf4100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatMute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatMute (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatMute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf41d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatOp.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatOp (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatOp___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf4370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatPassword.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatPassword (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatPassword___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf4510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatSetAFKMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatSetAFKMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatSetAFKMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf3e80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChatSetDNDMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChatSetDNDMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChatSetDNDMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf3f50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ChosenRewards.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ChosenRewards (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ChosenRewards___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d1b630(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ClientReady.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ClientReady (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ClientReady___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d93e00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_CombatDebug.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_CombatDebug (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_CombatDebug___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb3130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_CombatDebugVerbose.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_CombatDebugVerbose (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_CombatDebugVerbose___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb33d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ConfirmEffect.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ConfirmEffect (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ConfirmEffect___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94c20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Craft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Craft (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Craft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e44ea0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_CreateCharacter.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_CreateCharacter (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_CreateCharacter___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d34500(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DHD.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DHD (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DHD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8ff00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DebugAbilityOnMob.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DebugAbilityOnMob (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DebugAbilityOnMob___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92920(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DebugBehaviorsOnMob.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DebugBehaviorsOnMob (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DebugBehaviorsOnMob___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cbb0b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DebugEvents.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DebugEvents (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DebugEvents___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c929f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DebugInteract.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DebugInteract (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DebugInteract___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DebugPathsOnMob.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DebugPathsOnMob (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DebugPathsOnMob___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cbb350(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DeleteCharacter.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DeleteCharacter (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DeleteCharacter___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9a480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DeleteMailMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DeleteMailMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DeleteMailMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d99760(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Despawn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Despawn (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Despawn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DialogButtonChoice.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DialogButtonChoice (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DialogButtonChoice___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Disconnect.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Disconnect (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Disconnect___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9a720(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DuelChallenge.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DuelChallenge (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DuelChallenge___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DuelForfeit.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DuelForfeit (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DuelForfeit___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c948e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_DuelResponse.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_DuelResponse (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_DuelResponse___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_DuelResponse___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ad7580(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_EndMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_EndMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_EndMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d91400(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_EnterErrorAIState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_EnterErrorAIState (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_EnterErrorAIState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c949b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ExitErrorAIState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ExitErrorAIState (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ExitErrorAIState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GMRemoveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GMRemoveItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GMRemoveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9b6e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GetItemInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GetItemInfo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GetItemInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c902e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GetMobAttribute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GetMobAttribute (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GetMobAttribute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c926b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveAbility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c914d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveAllAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveAllAbilities (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveAllAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c921d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveAmmo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveAmmo (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveAmmo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91400(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveBlueprint.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveBlueprint (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveBlueprint___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d97290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveExpertise.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveExpertise (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveExpertise___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c945a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveFaction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveFaction (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveFaction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d96ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveGearset.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveGearset (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveGearset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d97530(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveInventory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveInventory (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveInventory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d977d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91330(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveMinigameContact.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveMinigameContact (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveMinigameContact___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveNaqahdah.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveNaqahdah (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveNaqahdah___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91260(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveRespawner.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveRespawner (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveRespawner___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c915a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveStargateAddress.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveStargateAddress (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveStargateAddress___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92ed0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveTrainingPoints.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveTrainingPoints (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveTrainingPoints___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c940c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GiveXp.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GiveXp (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GiveXp___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Goto.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Goto (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Goto___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d901a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GotoLocation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GotoLocation (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GotoLocation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d90440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_GotoXYZ.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_GotoXYZ (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_GotoXYZ___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f9f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_HealDebug.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_HealDebug (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_HealDebug___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb3670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_InitialResponse.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_InitialResponse (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_InitialResponse___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92780(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Interact.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Interact (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Interact___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d97a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Invisible.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Invisible (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Invisible___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92d30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Kill.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Kill (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Kill___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ListAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ListAbilities (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ListAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d938c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ListInteractions.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ListInteractions (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ListInteractions___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93f10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ListItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ListItems (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ListItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8fe00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadAbility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadAbilitySet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadAbilitySet (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadAbilitySet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadBehavior.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadBehavior (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadBehavior___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c936f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadConstants.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadConstants (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadConstants___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c933b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadInteractionSet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadInteractionSet (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadInteractionSet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93960(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadMOB.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadMOB (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadMOB___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c937c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadMission.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadMission (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadMission___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93a30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LoadNACSI.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LoadNACSI (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LoadNACSI___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93550(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LogOff.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LogOff (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LogOff___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9a9c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_LootItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_LootItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_LootItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d93620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MinigameCallAbort.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MinigameCallAbort (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MinigameCallAbort___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d92660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MinigameCallAccept.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MinigameCallAccept (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MinigameCallAccept___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d92900(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MinigameCallDecline.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MinigameCallDecline (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MinigameCallDecline___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d92ba0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MinigameCallRequest.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MinigameCallRequest (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MinigameCallRequest___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d923c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MinigameComplete.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MinigameComplete (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MinigameComplete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f780(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MinigameStartCancel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MinigameStartCancel (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MinigameStartCancel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d92120(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionAbandon.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionAbandon (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionAbandon___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90140(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionAdvance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionAdvance (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionAdvance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90e40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionAssign.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionAssign (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionAssign___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90bd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionClear.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionClear (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionClear___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90ca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionClearActive.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionClearActive (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionClearActive___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb5bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionClearHistory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionClearHistory (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionClearHistory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb5e90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionComplete.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionComplete (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionComplete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionDetails.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionDetails (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionDetails___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb6130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionListFull.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionListFull (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionListFull___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb63d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionReset.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionReset (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionReset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90f10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MissionSetAvailable.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MissionSetAvailable (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MissionSetAvailable___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c910c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MobData.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MobData (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MobData___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f5e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_MoveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_MoveItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_MoveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d94340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_OrganizationCreation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_OrganizationCreation (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_OrganizationCreation___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_OrganizationCreation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ad73e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_OrganizationInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_OrganizationInvite (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_OrganizationInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d94880(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_OrganizationKick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_OrganizationKick (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_OrganizationKick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d95300(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_OrganizationLeave.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_OrganizationLeave (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_OrganizationLeave___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d95060(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_OrganizationMOTD.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_OrganizationMOTD (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_OrganizationMOTD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d95840(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_OrganizationNote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_OrganizationNote (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_OrganizationNote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d95ae0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PayCODForMailMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PayCODForMailMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PayCODForMailMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9a1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PerfStats.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PerfStats (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PerfStats___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9ce80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PerfStatsByChannel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PerfStatsByChannel (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PerfStatsByChannel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94b50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PetAbilityToggle.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PetAbilityToggle (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PetAbilityToggle___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93ca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PetChangeStance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PetChangeStance (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PetChangeStance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d3ac10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PetInvokeAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PetInvokeAbility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PetInvokeAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93bd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Petition.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Petition (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Petition___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf45e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Physics.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Physics (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Physics___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92e00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PlayCharacter.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PlayCharacter (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PlayCharacter___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9ac60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PrintStats.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PrintStats (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PrintStats___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d90980(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_PurchaseItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_PurchaseItems (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_PurchaseItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d97d10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RechargeItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RechargeItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RechargeItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RechargeItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RechargeItems (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RechargeItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d987a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RegenerateCoverLinks.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RegenerateCoverLinks (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RegenerateCoverLinks___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93140(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ReloadInventory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ReloadInventory (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ReloadInventory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8fb90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ReloadOrganizations.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ReloadOrganizations (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ReloadOrganizations___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8fac0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RemoveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RemoveItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RemoveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d945e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RepairItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RepairItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RepairItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RepairItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RepairItems (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RepairItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d98500(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RequestAmmoChange.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RequestAmmoChange (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RequestAmmoChange___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c903b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RequestMailBody.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RequestMailBody (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RequestMailBody___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d99220(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RequestMailHeaders.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RequestMailHeaders (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RequestMailHeaders___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d98ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RequestReload.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RequestReload (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RequestReload___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92b90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RequestSpectateList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RequestSpectateList (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RequestSpectateList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d91be0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Research.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Research (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Research___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e44f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ResetAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ResetAbilities (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ResetAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Respawn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Respawn (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Respawn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb55f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Respec.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Respec (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Respec___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c922a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RespecAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RespecAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_RespecAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RespecAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ad7240(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_RespecCraft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_RespecCraft (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_RespecCraft___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_RespecCraft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ad7310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ReturnMailMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ReturnMailMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ReturnMailMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d99a00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ReverseEngineer.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ReverseEngineer (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ReverseEngineer___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e45040(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SellItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SellItems (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SellItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d97fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SendGMShout.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SendGMShout (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SendGMShout___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf46b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SendMailMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SendMailMessage (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SendMailMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d98f80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetAutoCycle.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetAutoCycle (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetAutoCycle___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cbbcb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetCrouched.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetCrouched (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetCrouched___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d93b60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetFaction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetFaction (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetFaction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d96d50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetFlag (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91dc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetFocus.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetFocus (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetFocus___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91c20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetFocusMax.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetFocusMax (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetFocusMax___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetGodMode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetGodMode (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetGodMode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91740(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetHealth.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetHealth (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetHealth___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetHealthMax.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetHealthMax (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetHealthMax___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91b50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetHideGM.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetHideGM (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetHideGM___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8fd30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetInstanceFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetInstanceFlag (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetInstanceFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94dc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetLevel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetLevel (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetLevel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92030(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetMobAbilitySet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetMobAbilitySet (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetMobAbilitySet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetMobAttribute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetMobAttribute (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetMobAttribute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c925e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetMobStance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetMobStance (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetMobStance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91e90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetMobVariable.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetMobVariable (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetMobVariable___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93b00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetMovementType.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetMovementType (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetMovementType___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92ac0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetNoAggro.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetNoAggro (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetNoAggro___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c918e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetNoXP.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetNoXP (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetNoXP___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetSpeed.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetSpeed (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetSpeed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c919b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetTarget.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetTarget (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetTarget___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c91f60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetTargetID.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetTargetID (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetTargetID___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9b440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SetTechSkill.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SetTechSkill (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SetTechSkill___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d96ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShareMission.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShareMission (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShareMission___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8ffa0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShareMissionResponse.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShareMissionResponse (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShareMissionResponse___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowFlag (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93e40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowIP.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowIP (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowIP___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowInstanceFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowInstanceFlag (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowInstanceFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowInventory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowInventory (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowInventory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90960(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowMobCount.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowMobCount (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowMobCount___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c907c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowNavigation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowNavigation (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowNavigation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94e90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowPlayer.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowPlayer (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowPlayer___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90a30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowPointSet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowPointSet (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowPointSet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c93070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowRotation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowRotation (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowRotation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90b00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ShowTargetLocation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ShowTargetLocation (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ShowTargetLocation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c906f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Spawn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Spawn (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Spawn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SpectateMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SpectateMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SpectateMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d91e80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SquadSetLootMode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SquadSetLootMode (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SquadSetLootMode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d96810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_StartMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_StartMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_StartMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d91160(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Summon.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Summon (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Summon___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d906e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_SystemOptions.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_SystemOptions (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_SystemOptions___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9cbe0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_TestLOS.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_TestLOS (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_TestLOS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9bc20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_ToggleCombatLOS.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_ToggleCombatLOS (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_ToggleCombatLOS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9bec0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_TradeLockState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_TradeLockState (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_TradeLockState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e26660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_TradeProposal.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_TradeProposal (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_TradeProposal___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e26590(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_TradeRequestCancel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_TradeRequestCancel (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_TradeRequestCancel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e264c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_TrainAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_TrainAbility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_TrainAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d98a40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Unstuck.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Unstuck (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Unstuck___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c944d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_UseAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_UseAbility (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_UseAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8fed0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_UseItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_UseItem (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_UseItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d940a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Users.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Users (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Users___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8fc60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_Who.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_Who (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_Who___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c90550(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_WorldInstanceReset.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_WorldInstanceReset (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_WorldInstanceReset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c94400(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_XRayEyes.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_XRayEyes (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_XRayEyes___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c92c60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_callForAid.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_callForAid (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_callForAid___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_callForAid___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00ae9670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_contactListCreate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_contactListCreate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_contactListCreate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5f4e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_contactListDelete.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_contactListDelete (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_contactListDelete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5f5b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_contactListRename.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_contactListRename (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_contactListRename___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e5f680(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_debugJoinMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_debugJoinMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_debugJoinMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cb4270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_debugStartMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_debugStartMinigame (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_debugStartMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f6b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_elementDataRequest.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_elementDataRequest (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_elementDataRequest___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00cf3b40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_onClientVersion.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_onClientVersion (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_onClientVersion___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d9dba0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_onDialGate.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_onDialGate (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_onDialGate___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d930e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_onSpaceQueueStatus.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_onSpaceQueueStatus (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_onSpaceQueueStatus___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00c8f510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_onStrikeTeamResponse.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_onStrikeTeamResponse (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_onStrikeTeamResponse___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d8eca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_requestItemData.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_requestItemData (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_requestItemData___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d40370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_NetOut_versionInfoRequest.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_NetOut_versionInfoRequest (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_NetOut_versionInfoRequest___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_NetOut_versionInfoRequest___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00421f40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_CamOptionChanged.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_CamOptionChanged (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Option_CamOptionChanged___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_CamOptionChanged___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005889f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_DevWindowedMode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_DevWindowedMode (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Option_DevWindowedMode___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_DevWindowedMode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00588270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_MasterVolume.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_MasterVolume (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_Option_MasterVolume___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_Option_MasterVolume___CallbackImpl at 0183c14c
   Also in vtable:
   CME_EventSignal_ZV5_PAX_AEXPBVEvent_Option_MasterVolume_P84_VSGWAudioDevice_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_Option_MasterVolume___CallbackImpl__vfunc_2(void)

{
  return &Event_Option_MasterVolume::RTTI_Type_Descriptor;
}


/* Also in vtable: CME_EventSignal_VEvent_Option_MasterVolume___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_MasterVolume___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00587ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_MusicVolume.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_MusicVolume (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_Option_MusicVolume___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_Option_MusicVolume___CallbackImpl at 0183c168
   Also in vtable:
   CME_EventSignal_ZV5_PAX_AEXPBVEvent_Option_MusicVolume_P84_VSGWAudioDevice_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_Option_MusicVolume___CallbackImpl__vfunc_2(void)

{
  return &Event_Option_MusicVolume::RTTI_Type_Descriptor;
}


/* Also in vtable: CME_EventSignal_VEvent_Option_MusicVolume___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_MusicVolume___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00588c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_Rendering.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_Rendering (2 functions)
 */

/* [VTable] CME_EventSignal_VEvent_Option_Rendering___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_Option_Rendering___CallbackImpl at 01840404
   Also in vtable:
   CME_EventSignal_ZV5_PAX_AEXPBVEvent_Option_Rendering_P84_VRenderThreadOptionManager_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_Option_Rendering___CallbackImpl__vfunc_2(void)

{
  return &Event_Option_Rendering::RTTI_Type_Descriptor;
}


/* Also in vtable: CME_EventSignal_VEvent_Option_Rendering___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_Rendering___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00588ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_Resolution.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_Resolution (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Option_Resolution___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_Resolution___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00588770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_ShowButtonBinds.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_ShowButtonBinds (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_ShowButtonBinds___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dc81e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Option_WindowedMode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Option_WindowedMode (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_Option_WindowedMode___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_Option_WindowedMode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005884f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Property_AppearanceLogging.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Property_AppearanceLogging (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_Property_AppearanceLogging___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dc86e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_Property_SctTextLevel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_Property_SctTextLevel (1 functions)
 */

undefined4 * __thiscall
CME_EventSignal_VEvent_Property_SctTextLevel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00dc8460(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_AbandonMission.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_AbandonMission (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_AbandonMission___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_AbandonMission___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a3070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_AbilityDebug.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_AbilityDebug (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_AbilityDebug___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_AbilityDebug___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a4470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChangeAmmo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChangeAmmo (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChangeAmmo___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChangeAmmo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a4bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChangeCoverWeight.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChangeCoverWeight (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChangeCoverWeight___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChangeCoverWeight___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ae6f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatBan.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatBan (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatBan___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatBan___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a6c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatFriend.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatFriend (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatFriend___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatFriend___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a5af0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatIgnore.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatIgnore (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatIgnore___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatIgnore___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a5870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatJoin.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatJoin (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatJoin___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatJoin___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a4e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatKick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatKick (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatKick___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatKick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a6770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatLeave.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatLeave (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatLeave___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatLeave___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a50f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatList (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatList___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a5ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatMute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatMute (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatMute___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatMute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a6270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatOp.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatOp (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatOp___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatOp___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a69f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatPassword.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatPassword (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatPassword___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatPassword___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a7170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatSetAFKMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatSetAFKMessage (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatSetAFKMessage___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatSetAFKMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a5370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatSetDNDMessage.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatSetDNDMessage (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatSetDNDMessage___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatSetDNDMessage___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a55f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatUnban.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatUnban (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatUnban___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatUnban___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a6ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatUnfriend.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatUnfriend (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatUnfriend___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatUnfriend___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a5d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChatUnmute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChatUnmute (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChatUnmute___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChatUnmute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a64f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ChooseOrgName.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ChooseOrgName (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ChooseOrgName___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ChooseOrgName___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ab4f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CombatDebug.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CombatDebug (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CombatDebug___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CombatDebug___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a3cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CombatDebugVerbose.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CombatDebugVerbose (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CombatDebugVerbose___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CombatDebugVerbose___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a3f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CommandDemote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CommandDemote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CommandDemote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CommandDemote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a91f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CommandInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CommandInvite (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CommandInvite___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CommandInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a82f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CommandKick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CommandKick (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CommandKick___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CommandKick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a8cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CommandLeave.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CommandLeave (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CommandLeave___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CommandLeave___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a8a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CommandMOTD.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CommandMOTD (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CommandMOTD___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CommandMOTD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a9470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_CommandPromote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_CommandPromote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_CommandPromote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_CommandPromote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a8f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ConfirmEffect.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ConfirmEffect (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ConfirmEffect___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ConfirmEffect___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b5770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DHD.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DHD (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DHD___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DHD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059f970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugAbilityOnMob.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugAbilityOnMob (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugAbilityOnMob___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugAbilityOnMob___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a05f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugEvents.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugEvents (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugEvents___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugEvents___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a0d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugFlash.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugFlash (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugFlash___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugFlash___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059f470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugInteract.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugInteract (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugInteract___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugInteract___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b0ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugJoinMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugJoinMinigame (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugJoinMinigame___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugJoinMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059e7f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugPathsOnMob.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugPathsOnMob (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugPathsOnMob___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugPathsOnMob___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a0af0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugPerformance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugPerformance (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugPerformance___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugPerformance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a0ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugStartMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugStartMinigame (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugStartMinigame___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugStartMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059e070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DebugTarget.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DebugTarget (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DebugTarget___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DebugTarget___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059ddf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DeleteItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DeleteItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DeleteItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DeleteItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00590470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Despawn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Despawn (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Despawn___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Despawn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a2170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DialogButtonChoice.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DialogButtonChoice (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DialogButtonChoice___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DialogButtonChoice___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a14f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DuelChallenge.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DuelChallenge (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DuelChallenge___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DuelChallenge___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b3970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DuelForfeit.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DuelForfeit (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DuelForfeit___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DuelForfeit___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b3e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DuelResponse.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DuelResponse (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DuelResponse___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DuelResponse___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b3bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_DumpObjects.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_DumpObjects (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_DumpObjects___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_DumpObjects___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b36f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Emote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Emote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Emote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Emote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058e3f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_EnterErrorAIState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_EnterErrorAIState (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_EnterErrorAIState___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_EnterErrorAIState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b40f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_EquipItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_EquipItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_EquipItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_EquipItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058fcf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Exit.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Exit (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Exit___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Exit___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059f6f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ExitErrorAIState.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ExitErrorAIState (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ExitErrorAIState___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ExitErrorAIState___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b4370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ForceClientCrash.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ForceClientCrash (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ForceClientCrash___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ForceClientCrash___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b22f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GMChat.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GMChat (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GMChat___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GMChat___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ada70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GMDeleteItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GMDeleteItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GMDeleteItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GMDeleteItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005906f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GMShout.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GMShout (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GMShout___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GMShout___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005adcf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GetItemInfo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GetItemInfo (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GetItemInfo___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GetItemInfo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a4970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GetMobAttribute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GetMobAttribute (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GetMobAttribute___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GetMobAttribute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a28f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00593b70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveAllAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveAllAbilities (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveAllAbilities___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveAllAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059a1f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveAmmo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveAmmo (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveAmmo___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveAmmo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005933f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveBlueprint.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveBlueprint (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveBlueprint___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveBlueprint___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00594570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveExpertise.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveExpertise (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveExpertise___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveExpertise___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b2f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveFaction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveFaction (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveFaction___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveFaction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00593670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveGearset.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveGearset (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveGearset___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveGearset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005947f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveInventory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveInventory (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveInventory___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveInventory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00594a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00593170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveNaqahdah.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveNaqahdah (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveNaqahdah___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveNaqahdah___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00592ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveRespawner.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveRespawner (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveRespawner___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveRespawner___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00594070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveTrainingPoints.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveTrainingPoints (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveTrainingPoints___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveTrainingPoints___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b18f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GiveXp.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GiveXp (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GiveXp___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GiveXp___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00592c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Goto.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Goto (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Goto___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Goto___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00594cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GotoLocation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GotoLocation (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GotoLocation___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GotoLocation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00594f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_GotoXYZ.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_GotoXYZ (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_GotoXYZ___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_GotoXYZ___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005951f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_HealDebug.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_HealDebug (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_HealDebug___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_HealDebug___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a41f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Help.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Help (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Help___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Help___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058d9f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_HelpFull.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_HelpFull (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_HelpFull___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_HelpFull___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058dc70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_InitialResponse.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_InitialResponse (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_InitialResponse___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_InitialResponse___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a1270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Interact.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Interact (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Interact___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Interact___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a0370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Invisible.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Invisible (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Invisible___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Invisible___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ad070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_InvokeAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_InvokeAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_InvokeAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_InvokeAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00593df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Kill.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Kill (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Kill___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Kill___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059abf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ListAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ListAbilities (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ListAbilities___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ListAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a2df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ListInteractions.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ListInteractions (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ListInteractions___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ListInteractions___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b13f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ListItems.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ListItems (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ListItems___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ListItems___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059fbf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005af0f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadAbilitySet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadAbilitySet (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadAbilitySet___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadAbilitySet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005af5f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadBehavior.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadBehavior (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadBehavior___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadBehavior___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005af870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadConstants.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadConstants (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadConstants___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadConstants___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aee70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadInteractionSet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadInteractionSet (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadInteractionSet___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadInteractionSet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005afd70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005afff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadMOB.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadMOB (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadMOB___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadMOB___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005afaf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadMission.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadMission (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadMission___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadMission___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b0270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LoadNACSI.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LoadNACSI (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LoadNACSI___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LoadNACSI___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005af370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Location.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Location (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Location___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Location___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059d8f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LogOff.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LogOff (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LogOff___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LogOff___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a1ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_LootItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_LootItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_LootItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_LootItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a2b70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MinigameComplete.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MinigameComplete (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MinigameComplete___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MinigameComplete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059ea70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionAbandon.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionAbandon (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionAbandon___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionAbandon___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a3a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionAdvance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionAdvance (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionAdvance___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionAdvance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00592270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionAssign.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionAssign (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionAssign___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionAssign___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005910f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionClear.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionClear (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionClear___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionClear___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00591370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionClearActive.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionClearActive (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionClearActive___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionClearActive___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005915f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionComplete.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionComplete (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionComplete___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionComplete___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00592770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionDetails.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionDetails (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionDetails___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionDetails___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00591ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionList.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionList (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionList___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionList___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00591af0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionListFull.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionListFull (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionListFull___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionListFull___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00591d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MissionReset.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MissionReset (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MissionReset___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MissionReset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005924f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MobData.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MobData (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MobData___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MobData___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059db70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_MoveItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_MoveItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_MoveItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_MoveItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a46f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_NTell.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_NTell (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_NTell___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_NTell___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058fa70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_PerfStatsByChannel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_PerfStatsByChannel (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_PerfStatsByChannel___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_PerfStatsByChannel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b4d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_PetAbilityToggle.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_PetAbilityToggle (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_PetAbilityToggle___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_PetAbilityToggle___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b0c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_PetInvokeAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_PetInvokeAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_PetInvokeAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_PetInvokeAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b0770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_PetInvokeCommand.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_PetInvokeCommand (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_PetInvokeCommand___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_PetInvokeCommand___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b09f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Petition.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Petition (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Petition___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Petition___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058f7f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Physics.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Physics (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Physics___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Physics___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ad2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_PrintStats.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_PrintStats (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_PrintStats___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_PrintStats___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00595e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_PurchaseItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_PurchaseItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_PurchaseItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_PurchaseItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a1c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_RechargeItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_RechargeItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_RechargeItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_RechargeItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a23f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Reload.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Reload (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Reload___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Reload___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aebf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ReloadInventory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ReloadInventory (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ReloadInventory___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ReloadInventory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005956f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_RepairItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_RepairItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_RepairItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_RepairItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00590970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ResetAbilities.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ResetAbilities (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ResetAbilities___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ResetAbilities___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00599f70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Respawn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Respawn (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Respawn___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Respawn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00590bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Respec.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Respec (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Respec___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Respec___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059a470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_RespecAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_RespecAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_RespecAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_RespecAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059a6f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_RespecCraft.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_RespecCraft (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_RespecCraft___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_RespecCraft___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059a970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Run.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Run (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Run___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Run___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005abef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Say.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Say (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Say___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Say___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058def0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SayChannel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SayChannel (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SayChannel___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SayChannel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058f2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SayCommand.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SayCommand (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SayCommand___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SayCommand___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058eb70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SayOfficer.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SayOfficer (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SayOfficer___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SayOfficer___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058edf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SayPlatoon.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SayPlatoon (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SayPlatoon___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SayPlatoon___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058f070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SaySquad.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SaySquad (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SaySquad___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SaySquad___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058e8f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SayTeam.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SayTeam (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SayTeam___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SayTeam___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058e670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetArchetype.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetArchetype (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetArchetype___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetArchetype___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005988f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetCommandNote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetCommandNote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetCommandNote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetCommandNote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a96f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetFaction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetFaction (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetFaction___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetFaction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005938f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetFlag (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetFlag___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00599570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetFly.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetFly (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetFly___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetFly___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005979f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetFocus.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetFocus (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetFocus___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetFocus___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00599070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetFocusMax.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetFocusMax (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetFocusMax___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetFocusMax___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005992f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetGhost.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetGhost (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetGhost___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetGhost___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00597c70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetGodMode.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetGodMode (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetGodMode___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetGodMode___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005960f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetHealth.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetHealth (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetHealth___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetHealth___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00598b70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetHealthMax.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetHealthMax (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetHealthMax___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetHealthMax___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00598df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetHideGM.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetHideGM (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetHideGM___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetHideGM___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00595bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetIgnoreFocus.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetIgnoreFocus (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetIgnoreFocus___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetIgnoreFocus___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005974f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetIgnoreHealth.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetIgnoreHealth (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetIgnoreHealth___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetIgnoreHealth___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00597770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetInfiniteAmmo.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetInfiniteAmmo (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetInfiniteAmmo___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetInfiniteAmmo___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00596af0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetInstanceFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetInstanceFlag (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetInstanceFlag___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetInstanceFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b5270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetInvulnerable.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetInvulnerable (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetInvulnerable___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetInvulnerable___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00596d70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetLevel.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetLevel (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetLevel___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetLevel___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00599cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetMobAbilitySet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetMobAbilitySet (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetMobAbilitySet___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetMobAbilitySet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b1670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetMobAttribute.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetMobAttribute (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetMobAttribute___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetMobAttribute___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a2670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetMobStance.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetMobStance (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetMobStance___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetMobStance___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005997f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetMobVariable.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetMobVariable (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetMobVariable___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetMobVariable___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b04f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetNoAggro.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetNoAggro (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetNoAggro___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetNoAggro___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00596870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetNoTarget.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetNoTarget (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetNoTarget___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetNoTarget___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00598170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetNoXP.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetNoXP (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetNoXP___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetNoXP___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00596370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}



/* === Additional global functions for CME_EventSignal_VEvent_SlashCmd_SetNoXp (1 functions) === */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetNoXp___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetNoXp___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00596ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetOmnipotent.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetOmnipotent (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetOmnipotent___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetOmnipotent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00597270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetPVP.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetPVP (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetPVP___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetPVP___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005983f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetSpectator.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetSpectator (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetSpectator___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetSpectator___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00597ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetSpeed.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetSpeed (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetSpeed___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetSpeed___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00598670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetTarget.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetTarget (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetTarget___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetTarget___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00599a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetTeamNote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetTeamNote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetTeamNote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetTeamNote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aaff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetTeamOfficerNote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetTeamOfficerNote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetTeamOfficerNote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetTeamOfficerNote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ab270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SetTechSkill.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SetTechSkill (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SetTechSkill___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SetTechSkill___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005942f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShareMission.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShareMission (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShareMission___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShareMission___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a32f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShareMissionAccept.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShareMissionAccept (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShareMissionAccept___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShareMissionAccept___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a3570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowArea.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowArea (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowArea___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowArea___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ac8f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowCover.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowCover (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowCover___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowCover___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ac170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowDialog.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowDialog (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowDialog___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowDialog___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a1770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowFPS.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowFPS (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowFPS___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowFPS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059b870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowFlag (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowFlag___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b1170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowIP.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowIP (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowIP___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowIP___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059baf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowInstanceFlag.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowInstanceFlag (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowInstanceFlag___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowInstanceFlag___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b4ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowInventory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowInventory (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowInventory___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowInventory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059bd70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowLog.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowLog (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowLog___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowLog___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059b370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowMemory.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowMemory (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowMemory___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowMemory___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059d670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowMobCount.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowMobCount (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowMobCount___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowMobCount___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059b5f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowMobPaths.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowMobPaths (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowMobPaths___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowMobPaths___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059cc70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowNavMesh.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowNavMesh (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowNavMesh___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowNavMesh___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059c9f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowNavigation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowNavigation (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowNavigation___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowNavigation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b54f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowPlayer.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowPlayer (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowPlayer___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowPlayer___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059bff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowPosition.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowPosition (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowPosition___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowPosition___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059d170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowRegion.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowRegion (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowRegion___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowRegion___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005acb70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowRotation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowRotation (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowRotation___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowRotation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059d3f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowSpawnSet.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowSpawnSet (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowSpawnSet___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowSpawnSet___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ac670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowSpawns.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowSpawns (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowSpawns___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowSpawns___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059c4f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowTargetLocation.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowTargetLocation (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowTargetLocation___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowTargetLocation___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059cef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowTriggers.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowTriggers (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowTriggers___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowTriggers___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059c270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ShowWaypoints.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ShowWaypoints (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ShowWaypoints___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ShowWaypoints___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059c770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SpaceQueueStatus.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SpaceQueueStatus (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SpaceQueueStatus___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SpaceQueueStatus___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b45f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Spawn.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Spawn (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Spawn___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Spawn___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059ae70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SquadInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SquadInvite (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SquadInvite___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SquadInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a73f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SquadInviteAccept.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SquadInviteAccept (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SquadInviteAccept___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SquadInviteAccept___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a7670(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SquadInviteDecline.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SquadInviteDecline (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SquadInviteDecline___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SquadInviteDecline___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a78f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SquadKick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SquadKick (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SquadKick___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SquadKick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a7b70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SquadLeave.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SquadLeave (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SquadLeave___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SquadLeave___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a8070(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_SquadPromote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_SquadPromote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_SquadPromote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_SquadPromote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a7df0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_StartMinigame.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_StartMinigame (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_StartMinigame___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_StartMinigame___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059f1f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Summon.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Summon (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Summon___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Summon___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0059b0f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamDemote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamDemote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamDemote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamDemote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aaaf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamInvite.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamInvite (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamInvite___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamInvite___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a9bf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamInviteAccept.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamInviteAccept (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamInviteAccept___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamInviteAccept___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a9e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamInviteDecline.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamInviteDecline (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamInviteDecline___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamInviteDecline___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aa0f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamKick.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamKick (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamKick___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamKick___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aa5f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamLeave.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamLeave (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamLeave___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamLeave___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aa370(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamMOTD.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamMOTD (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamMOTD___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamMOTD___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aad70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TeamPromote.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TeamPromote (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TeamPromote___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TeamPromote___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005aa870(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Tell.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Tell (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Tell___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Tell___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058f570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TestLOS.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TestLOS (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TestLOS___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TestLOS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ae1f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TestSequence.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TestSequence (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TestSequence___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TestSequence___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00590e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TimeofDay.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TimeofDay (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TimeofDay___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TimeofDay___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a00f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_ToggleCombatLOS.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_ToggleCombatLOS (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_ToggleCombatLOS___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_ToggleCombatLOS___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005adf70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_TrainAbility.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_TrainAbility (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_TrainAbility___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_TrainAbility___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005a19f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_UnequipItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_UnequipItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_UnequipItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_UnequipItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058ff70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Unstuck.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Unstuck (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Unstuck___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Unstuck___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b2cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_UseItem.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_UseItem (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_UseItem___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_UseItem___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005901f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Users.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Users (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Users___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Users___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00595970(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Walk.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Walk (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Walk___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Walk___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005abc70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Who.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Who (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Who___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Who___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005ab770(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_WorldInstanceReset.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_WorldInstanceReset (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_WorldInstanceReset___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_WorldInstanceReset___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005b2a70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_XRayEyes.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_XRayEyes (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_XRayEyes___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_XRayEyes___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005acdf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_SlashCmd_Yell.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_SlashCmd_Yell (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_SlashCmd_Yell___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_SlashCmd_Yell___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_0058e170(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VEvent_UI_BindableAction.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_UI_BindableAction (2 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_UI_BindableAction___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_UI_BindableAction___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_005740d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CME_EventSignal_VEvent_UI_BindableAction___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_UI_BindableAction___CallbackImpl at 018facdc
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_UI_BindableAction_P845_ScriptedWindow_UActionEventHandler_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_UI_BindableAction___CallbackImpl__vfunc_2(void)

{
  return &Event_UI_BindableAction::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_VEvent_UI_SlashCommand.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_UI_SlashCommand (2 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_UI_SlashCommand___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_UI_SlashCommand___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00573e50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] CME_EventSignal_VEvent_UI_SlashCommand___CallbackImpl virtual function #2
   VTable: vtable_CME_EventSignal_VEvent_UI_SlashCommand___CallbackImpl at 018facc0
   Also in vtable:
   CME_EventSignal_ZV6_PAX_AEXPBVEvent_UI_SlashCommand_P845_ScriptedWindow_UCommandEventHandler_CME_EventSignal_UNoSubject___MemberCallback
   (slot 2) */

TypeDescriptor * CME_EventSignal_VEvent_UI_SlashCommand___CallbackImpl__vfunc_2(void)

{
  return &Event_UI_SlashCommand::RTTI_Type_Descriptor;
}




/* ========== CME_EventSignal_VEvent_World_Loaded.c ========== */

/*
 * SGW.exe - CME_EventSignal_VEvent_World_Loaded (1 functions)
 */

/* Also in vtable: CME_EventSignal_VEvent_World_Loaded___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VEvent_World_Loaded___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00554940(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_EventSignal_VNetworkEvent.c ========== */

/*
 * SGW.exe - CME_EventSignal_VNetworkEvent (1 functions)
 */

/* Also in vtable: CME_EventSignal_VNetworkEvent___TypedEmitInfo (slot 0) */

undefined4 * __thiscall
CME_EventSignal_VNetworkEvent___TypedEmitInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00441430(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_Exception.c ========== */

/*
 * SGW.exe - CME_Exception (2 functions)
 */

/* [VTable] CME_Exception virtual function #1
   VTable: vtable_CME_Exception at 0192520c */

int __fastcall CME_Exception__vfunc_1(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 0xc);
  if (0xf < *(uint *)(iVar1 + 0x20)) {
    return *(int *)(iVar1 + 0xc);
  }
  return iVar1 + 0xc;
}


/* Also in vtable: CME_Exception (slot 0) */

exception * __thiscall CME_Exception__vfunc_0(void *this,byte param_1)

{
  FUN_00a52d40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_Exception_ExceptionImpl.c ========== */

/*
 * SGW.exe - CME_Exception_ExceptionImpl (1 functions)
 */

/* Also in vtable: CME_Exception_ExceptionImpl (slot 0) */

undefined4 * __thiscall CME_Exception_ExceptionImpl__vfunc_0(void *this,byte param_1)

{
  FUN_00a55910(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_FindCoverNodeCentroid.c ========== */

/*
 * SGW.exe - CME_FindCoverNodeCentroid (1 functions)
 */

undefined4 * __thiscall CME_FindCoverNodeCentroid__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this =
       CME::KMeans<class_CME::CoverNode*,class_Vector3>::FindCentroidFunctor::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_GenericEvent.c ========== */

/*
 * SGW.exe - CME_GenericEvent (1 functions)
 */

/* Also in vtable: CME_GenericEvent (slot 0) */

undefined4 * __thiscall CME_GenericEvent__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167a013;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::GenericEvent::vftable;
  *(undefined ***)this = CME::BasicPropertyTree<struct_TypeList::NullType>::vftable;
  local_4 = 0xffffffff;
  Detail__unknown_00440a50((uint *)((int)this + 4));
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== CME_InternalTextCmd.c ========== */

/*
 * SGW.exe - CME_InternalTextCmd (1 functions)
 */

/* Also in vtable: CME_InternalTextCmd (slot 0) */

undefined4 * __thiscall CME_InternalTextCmd__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016d66c8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::InternalTextCmd::vftable;
  local_4 = 0xffffffff;
  FUN_00a4f730(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== CME_TEXT_CMD_TOO_FEW_PARAMS.c ========== */

/*
 * SGW.exe - CME_TEXT_CMD_TOO_FEW_PARAMS (2 functions)
 */

/* [VTable] CME_TEXT_CMD_TOO_FEW_PARAMS virtual function #3
   VTable: vtable_CME_TEXT_CMD_TOO_FEW_PARAMS at 0192513c */

void __thiscall CME_TEXT_CMD_TOO_FEW_PARAMS__vfunc_3(void *this,void *param_1)

{
  FUN_00a372f0(param_1,*(uint *)((int)this + 8),this,&TypeDescriptor_01dab4cc,
               (type_info *)&CME::TEXT_CMD_TOO_FEW_PARAMS::RTTI_Type_Descriptor);
  return;
}


/* [VTable] CME_TEXT_CMD_TOO_FEW_PARAMS virtual function #2
   VTable: vtable_CME_TEXT_CMD_TOO_FEW_PARAMS at 0192513c */

void __thiscall CME_TEXT_CMD_TOO_FEW_PARAMS__vfunc_2(void *this,void *param_1,undefined4 param_2)

{
  FUN_00a51850(param_1,*(int *)((int)this + 8),this,param_2);
  return;
}




/* ========== CME_TextCmd.c ========== */

/*
 * SGW.exe - CME_TextCmd (1 functions)
 */

/* Also in vtable: CME_TextCmd (slot 0) */

undefined4 * __thiscall CME_TextCmd__vfunc_0(void *this,byte param_1)

{
  FUN_00a4f730(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_TextCommandManager.c ========== */

/*
 * SGW.exe - CME_TextCommandManager (2 functions)
 */

/* [VTable] CME_TextCommandManager virtual function #1
   VTable: vtable_CME_TextCommandManager at 01925158 */

undefined4 CME_TextCommandManager__vfunc_1(void)

{
  undefined4 uVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016d63ab;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  uVar1 = FUN_005738f0((uint *)&stack0x00000004);
  ExceptionList = local_c;
  return CONCAT31((int3)((uint)uVar1 >> 8),1);
}


/* Also in vtable: CME_TextCommandManager (slot 0) */

undefined4 * __thiscall CME_TextCommandManager__vfunc_0(void *this,byte param_1)

{
  FUN_00a4e1b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CME_TypeList_UNullType.c ========== */

/*
 * SGW.exe - CME_TypeList_UNullType (2 functions)
 */

/* Also in vtable: CME_TypeList_UNullType___BasicPropertyList (slot 0) */

undefined4 * __thiscall CME_TypeList_UNullType___BasicPropertyList__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01679e9b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::BasicPropertyList<struct_TypeList::NullType>::vftable;
  local_4 = 0xffffffff;
  Detail__unknown_0043d770((uint *)((int)this + 4));
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


/* Also in vtable: CME_TypeList_UNullType___BasicPropertyTree (slot 0) */

undefined4 * __thiscall CME_TypeList_UNullType___BasicPropertyTree__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0168f42b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = CME::BasicPropertyTree<struct_TypeList::NullType>::vftable;
  local_4 = 0xffffffff;
  Detail__unknown_00440a50((uint *)((int)this + 4));
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== CME_UIScreen.c ========== */

/*
 * SGW.exe - CME_UIScreen (59 functions)
 */

/* [VTable] CMETextCmds_cmd__OptionalParamType virtual function #4
   VTable: vtable_CMETextCmds_cmd__OptionalParamType at 01926cac */

void __thiscall
CMETextCmds_cmd__OptionalParamType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00a5f7d0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] CMETextCmds_cmd__MandatoryParamType virtual function #4
   VTable: vtable_CMETextCmds_cmd__MandatoryParamType at 01926c88 */

void __thiscall
CMETextCmds_cmd__MandatoryParamType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00a5f8c0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] CMETextCmds_cmd__CommandType virtual function #4
   VTable: vtable_CMETextCmds_cmd__CommandType at 01926cf4 */

void __thiscall
CMETextCmds_cmd__CommandType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00a60b60(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] CMETextCmds_cmd__OptionalParamType virtual function #6
   VTable: vtable_CMETextCmds_cmd__OptionalParamType at 01926cac */

void __thiscall
CMETextCmds_cmd__OptionalParamType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00a62240(param_1,param_2,this,param_3);
  return;
}


/* [VTable] CMETextCmds_cmd__MandatoryParamType virtual function #6
   VTable: vtable_CMETextCmds_cmd__MandatoryParamType at 01926c88 */

void __thiscall
CMETextCmds_cmd__MandatoryParamType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00a623f0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] CMETextCmds_cmd__OptionalParamType virtual function #5
   VTable: vtable_CMETextCmds_cmd__OptionalParamType at 01926cac */

int * __thiscall
CMETextCmds_cmd__OptionalParamType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00a62240(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    CMETextCmds_cmd__unknown_00a63340(param_1);
  }
  return piVar1;
}


/* [VTable] CMETextCmds_cmd__MandatoryParamType virtual function #5
   VTable: vtable_CMETextCmds_cmd__MandatoryParamType at 01926c88 */

int * __thiscall
CMETextCmds_cmd__MandatoryParamType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00a623f0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    CMETextCmds_cmd__unknown_00a63340(param_1);
  }
  return piVar1;
}


/* [VTable] cme_hwprofile_cmehwprofile__Config virtual function #4
   VTable: vtable_cme_hwprofile_cmehwprofile__Config at 01929ecc */

undefined4 __thiscall
cme_hwprofile_cmehwprofile__Config__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,0xf);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00a71240(param_1,"cmehwprofile:device",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionSaveType virtual function #4
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionSaveType at 019644b4 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionSaveType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b043c0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionGroupType virtual function #4
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionGroupType at 01964490 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionGroupType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b04550(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionValueType virtual function #4
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionValueType at 0196446c */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionValueType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b04690(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__OptionGroupType_option virtual function #4
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__OptionGroupType_option at 019644d8 */

void __thiscall
SGWSystemOptions__SGWSystemOptions__OptionGroupType_option__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b064b0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__savedOptions virtual function #4
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__savedOptions at 0183fe80 */

undefined4 __thiscall
SGWSystemOptions__SGWSystemOptions__savedOptions__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,0xc);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00b06070(param_1,"options",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__optionGroups virtual function #4
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__optionGroups at 0183fe5c */

undefined4 __thiscall
SGWSystemOptions__SGWSystemOptions__optionGroups__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,0xb);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00b06190(param_1,"optionGroup",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionSaveType virtual function #6
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionSaveType at 019644b4 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionSaveType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b08440(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionValueType virtual function #6
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionValueType at 0196446c */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionValueType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b086c0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__OptionGroupType_option virtual function #6
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__OptionGroupType_option at 019644d8 */

void __thiscall
SGWSystemOptions__SGWSystemOptions__OptionGroupType_option__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b09590(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionGroupType virtual function #6
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionGroupType at 01964490 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionGroupType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b09c00(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__OptionGroupType_option virtual function #5
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__OptionGroupType_option at 019644d8 */

int * __thiscall
SGWSystemOptions__SGWSystemOptions__OptionGroupType_option__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b09590(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWSystemOptions_SGWSystemOptions__unknown_00b0a8f0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionSaveType virtual function #5
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionSaveType at 019644b4 */

int * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionSaveType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b08440(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWSystemOptions_SGWSystemOptions__unknown_00b0a8f0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionGroupType virtual function #5
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionGroupType at 01964490 */

int * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionGroupType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b09c00(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWSystemOptions_SGWSystemOptions__unknown_00b0a8f0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionValueType virtual function #5
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionValueType at 0196446c */

int * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionValueType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b086c0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWSystemOptions_SGWSystemOptions__unknown_00b0a8f0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action_defaultBind virtual function #4
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action_defaultBind at 01964be4 */

void __thiscall
SGWBindableActions__action__ActionGroupType_action_defaultBind__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b0b9f0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action virtual function #4
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action at 01964c08 */

void __thiscall
SGWBindableActions__action__ActionGroupType_action__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b0bb60(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWBindableActions_action__KeyBindType virtual function #4
   VTable: vtable_SGWBindableActions_action__KeyBindType at 01964c50 */

void __thiscall
SGWBindableActions_action__KeyBindType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b0bd80(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWBindableActions_action__ActionGroupType virtual function #4
   VTable: vtable_SGWBindableActions_action__ActionGroupType at 01964c2c */

void __thiscall
SGWBindableActions_action__ActionGroupType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b0bed0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWBindableActions__action__SavedBindings virtual function #4
   VTable: vtable_SGWBindableActions__action__SavedBindings at 01840734 */

undefined4 __thiscall
SGWBindableActions__action__SavedBindings__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,0xb);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00b0d1b0(param_1,"bindings",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWBindableActions__action__actionGroups virtual function #4
   VTable: vtable_SGWBindableActions__action__actionGroups at 01840710 */

undefined4 __thiscall
SGWBindableActions__action__actionGroups__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,10);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00b0d2d0(param_1,"actionGroup",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action_defaultBind virtual function #6
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action_defaultBind at 01964be4 */

void __thiscall
SGWBindableActions__action__ActionGroupType_action_defaultBind__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b0eed0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action virtual function #6
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action at 01964c08 */

void __thiscall
SGWBindableActions__action__ActionGroupType_action__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b0f0a0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWBindableActions_action__KeyBindType virtual function #6
   VTable: vtable_SGWBindableActions_action__KeyBindType at 01964c50 */

void __thiscall
SGWBindableActions_action__KeyBindType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b0f6b0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWBindableActions_action__ActionGroupType virtual function #6
   VTable: vtable_SGWBindableActions_action__ActionGroupType at 01964c2c */

void __thiscall
SGWBindableActions_action__ActionGroupType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b0f880(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action_defaultBind virtual function #5
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action_defaultBind at 01964be4 */

int * __thiscall
SGWBindableActions__action__ActionGroupType_action_defaultBind__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b0eed0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWBindableActions__unknown_00b11060(param_1);
  }
  return piVar1;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action virtual function #5
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action at 01964c08 */

int * __thiscall
SGWBindableActions__action__ActionGroupType_action__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b0f0a0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWBindableActions__unknown_00b11060(param_1);
  }
  return piVar1;
}


/* [VTable] SGWBindableActions_action__KeyBindType virtual function #5
   VTable: vtable_SGWBindableActions_action__KeyBindType at 01964c50 */

int * __thiscall
SGWBindableActions_action__KeyBindType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b0f6b0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWBindableActions__unknown_00b11060(param_1);
  }
  return piVar1;
}


/* [VTable] SGWBindableActions_action__ActionGroupType virtual function #5
   VTable: vtable_SGWBindableActions_action__ActionGroupType at 01964c2c */

int * __thiscall
SGWBindableActions_action__ActionGroupType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b0f880(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWBindableActions__unknown_00b11060(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action_defaultBind virtual function #4
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action_defaultBind at 01965cec */

void __thiscall
SGWTableOfContents__action__ActionGroupType_action_defaultBind__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b12230(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action virtual function #4
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action at 01965d10 */

void __thiscall
SGWTableOfContents__action__ActionGroupType_action__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b123a0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents_action__KeyBindType virtual function #4
   VTable: vtable_SGWTableOfContents_action__KeyBindType at 01965d58 */

void __thiscall
SGWTableOfContents_action__KeyBindType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b12650(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents_action__ActionGroupType virtual function #4
   VTable: vtable_SGWTableOfContents_action__ActionGroupType at 01965d34 */

void __thiscall
SGWTableOfContents_action__ActionGroupType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b127a0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents_cmd__OptionalParamType virtual function #4
   VTable: vtable_SGWTableOfContents_cmd__OptionalParamType at 01965c80 */

void __thiscall
SGWTableOfContents_cmd__OptionalParamType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b129c0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents_cmd__MandatoryParamType virtual function #4
   VTable: vtable_SGWTableOfContents_cmd__MandatoryParamType at 01965c5c */

void __thiscall
SGWTableOfContents_cmd__MandatoryParamType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b12ab0(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents__action__SavedBindings virtual function #4
   VTable: vtable_SGWTableOfContents__action__SavedBindings at 01965de8 */

undefined4 __thiscall
SGWTableOfContents__action__SavedBindings__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,0x14);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00b14e30(param_1,"bindings",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWTableOfContents__action__actionGroups virtual function #4
   VTable: vtable_SGWTableOfContents__action__actionGroups at 01965dc4 */

undefined4 __thiscall
SGWTableOfContents__action__actionGroups__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a3b160(param_1,param_3,(uint)this,0x13);
  CME_UIScreen__unknown_00a4a350(param_1,param_2,iVar1,param_4);
  FUN_00b15390(param_1,"actionGroup",-1,(int)this + 4);
  CME_UIScreen__unknown_00a45b20(param_1,param_2);
  return 0;
}


/* [VTable] SGWTableOfContents_cmd__CommandType virtual function #4
   VTable: vtable_SGWTableOfContents_cmd__CommandType at 01965d7c */

void __thiscall
SGWTableOfContents_cmd__CommandType__vfunc_4
          (void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  CME_UIScreen__unknown_00b15820(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action_defaultBind virtual function #6
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action_defaultBind at 01965cec */

void __thiscall
SGWTableOfContents__action__ActionGroupType_action_defaultBind__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b19180(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action virtual function #6
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action at 01965d10 */

void __thiscall
SGWTableOfContents__action__ActionGroupType_action__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b19350(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents_action__KeyBindType virtual function #6
   VTable: vtable_SGWTableOfContents_action__KeyBindType at 01965d58 */

void __thiscall
SGWTableOfContents_action__KeyBindType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b19da0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents_action__ActionGroupType virtual function #6
   VTable: vtable_SGWTableOfContents_action__ActionGroupType at 01965d34 */

void __thiscall
SGWTableOfContents_action__ActionGroupType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b19f70(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents_cmd__OptionalParamType virtual function #6
   VTable: vtable_SGWTableOfContents_cmd__OptionalParamType at 01965c80 */

void __thiscall
SGWTableOfContents_cmd__OptionalParamType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b1a670(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents_cmd__MandatoryParamType virtual function #6
   VTable: vtable_SGWTableOfContents_cmd__MandatoryParamType at 01965c5c */

void __thiscall
SGWTableOfContents_cmd__MandatoryParamType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b1a820(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents_cmd__CommandType virtual function #6
   VTable: vtable_SGWTableOfContents_cmd__CommandType at 01965d7c */

void __thiscall
SGWTableOfContents_cmd__CommandType__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  CME_UIScreen__unknown_00b1c590(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action_defaultBind virtual function #5
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action_defaultBind at 01965cec */

int * __thiscall
SGWTableOfContents__action__ActionGroupType_action_defaultBind__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b19180(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action virtual function #5
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action at 01965d10 */

int * __thiscall
SGWTableOfContents__action__ActionGroupType_action__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b19350(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents_action__KeyBindType virtual function #5
   VTable: vtable_SGWTableOfContents_action__KeyBindType at 01965d58 */

int * __thiscall
SGWTableOfContents_action__KeyBindType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b19da0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents_action__ActionGroupType virtual function #5
   VTable: vtable_SGWTableOfContents_action__ActionGroupType at 01965d34 */

int * __thiscall
SGWTableOfContents_action__ActionGroupType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b19f70(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents_cmd__CommandType virtual function #5
   VTable: vtable_SGWTableOfContents_cmd__CommandType at 01965d7c */

int * __thiscall
SGWTableOfContents_cmd__CommandType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b1c590(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents_cmd__OptionalParamType virtual function #5
   VTable: vtable_SGWTableOfContents_cmd__OptionalParamType at 01965c80 */

int * __thiscall
SGWTableOfContents_cmd__OptionalParamType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b1a670(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents_cmd__MandatoryParamType virtual function #5
   VTable: vtable_SGWTableOfContents_cmd__MandatoryParamType at 01965c5c */

int * __thiscall
SGWTableOfContents_cmd__MandatoryParamType__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = CME_UIScreen__unknown_00b1a820(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}




/* ========== CME_X.c ========== */

/*
 * SGW.exe - CME_X (2 functions)
 */

/* [VTable] CME_GenericEvent virtual function #1
   VTable: vtable_CME_GenericEvent at 017fb950
   Also in vtable: CME_X___SubjectEvent (slot 1)
   Also in vtable: NetworkEvent (slot 1)
   Also in vtable: Event_World_Loaded (slot 1)
   Also in vtable: Event_UI_SlashCommand (slot 1)
   Also in vtable: Event_UI_BindableAction (slot 1)
   Also in vtable: Event_Action_MouseClick (slot 1)
   Also in vtable: Event_Action_MouseLook (slot 1)
   Also in vtable: Event_Action_MoveForward (slot 1)
   Also in vtable: Event_Action_MoveBack (slot 1)
   Also in vtable: Event_Action_StepLeft (slot 1)
   Also in vtable: Event_Action_StepRight (slot 1)
   Also in vtable: Event_Action_TurnLeft (slot 1)
   Also in vtable: Event_Action_TurnRight (slot 1)
   Also in vtable: Event_Action_Jump (slot 1)
   Also in vtable: Event_Action_Crouch (slot 1)
   Also in vtable: Event_Action_Walk (slot 1)
   Also in vtable: Event_Action_ZoomIn (slot 1)
   Also in vtable: Event_Action_ZoomOut (slot 1)
   Also in vtable: Event_Action_CancelTarget (slot 1)
   Also in vtable: Event_Action_CycleTargetForward (slot 1)
   Also in vtable: Event_Action_CycleTargetBackward (slot 1)
   Also in vtable: Event_Action_CycleFriendlyTargetForward (slot 1)
   Also in vtable: Event_Action_CycleFriendlyTargetBackward (slot 1)
   Also in vtable: Event_Action_TargetSelf (slot 1)
   Also in vtable: Event_Action_TargetSquadMember1 (slot 1)
   Also in vtable: Event_Action_TargetSquadMember2 (slot 1)
   Also in vtable: Event_Action_TargetSquadMember3 (slot 1)
   Also in vtable: Event_Action_TargetSquadMember4 (slot 1)
   Also in vtable: Event_Action_TargetSquadMember5 (slot 1)
   Also in vtable: Event_Action_TargetSquadMember6 (slot 1)
   Also in vtable: Event_Action_TargetPet (slot 1)
   Also in vtable: Event_Action_TargetPartyPet1 (slot 1)
   Also in vtable: Event_Action_TargetPartyPet2 (slot 1)
   Also in vtable: Event_Action_TargetPartyPet3 (slot 1)
   Also in vtable: Event_Action_TargetPartyPet4 (slot 1)
   Also in vtable: Event_Action_TargetPartyPet5 (slot 1)
   Also in vtable: Event_Action_TargetPartyPet6 (slot 1)
   Also in vtable: Event_Action_ToggleAutorun (slot 1)
   Also in vtable: Event_Option_MasterVolume (slot 1)
   Also in vtable: Event_Option_DevWindowedMode (slot 1)
   Also in vtable: Event_Option_WindowedMode (slot 1)
   Also in vtable: Event_Option_Resolution (slot 1)
   Also in vtable: Event_Option_CamOptionChanged (slot 1)
   Also in vtable: Event_Option_MusicVolume (slot 1)
   Also in vtable: Event_Option_Rendering (slot 1)
   Also in vtable: Event_Editor_TestSequence (slot 1)
   Also in vtable: Event_Editor_Close (slot 1)
   Also in vtable: Event_Editor_SequenceBegin (slot 1)
   Also in vtable: Event_Editor_SequenceInterrupt (slot 1)
   Also in vtable: Event_Editor_SequenceEnd (slot 1)
   Also in vtable: Event_Editor_TogglePhysicsMode (slot 1)
   Also in vtable: Event_Editor_ViewWireframe (slot 1)
   Also in vtable: Event_Editor_ViewUnlit (slot 1)
   Also in vtable: Event_Editor_ViewLit (slot 1)
   Also in vtable: Event_Editor_ShowPerformance (slot 1)
   Also in vtable: Event_Editor_ShowFPS (slot 1)
   Also in vtable: Event_Editor_ScreenShot (slot 1)
   Also in vtable: Event_Editor_ShadowStats (slot 1)
   Also in vtable: Event_Editor_CameraDefault (slot 1)
   Also in vtable: Event_Editor_Camera1stPerson (slot 1)
   Also in vtable: Event_Editor_Camera3rdPerson (slot 1)
   Also in vtable: Event_Editor_CameraFixed (slot 1)
   Also in vtable: Event_Editor_CameraFixedTracking (slot 1)
   Also in vtable: Event_Editor_CameraFree (slot 1)
   Also in vtable: Event_Editor_Ghost (slot 1)
   Also in vtable: Event_Editor_Walk (slot 1)
   Also in vtable: Event_Editor_Use (slot 1)
   Also in vtable: Event_Editor_ToggleCombat (slot 1)
   Also in vtable: Event_Editor_SetPIEScript1Active (slot 1)
   Also in vtable: Event_Editor_SetPIEScript2Active (slot 1)
   Also in vtable: Event_Editor_PIEScriptLoad (slot 1)
   Also in vtable: Event_Editor_PIEScriptStart (slot 1)
   Also in vtable: Event_Editor_PIEScriptTogglePause (slot 1)
   Also in vtable: Event_Editor_PIEScriptStep (slot 1)
   Also in vtable: Event_SlashCmd_Help (slot 1)
   Also in vtable: Event_SlashCmd_HelpFull (slot 1)
   Also in vtable: Event_SlashCmd_Say (slot 1)
   Also in vtable: Event_SlashCmd_Yell (slot 1)
   Also in vtable: Event_SlashCmd_Emote (slot 1)
   Also in vtable: Event_SlashCmd_SayTeam (slot 1)
   Also in vtable: Event_SlashCmd_SaySquad (slot 1)
   Also in vtable: Event_SlashCmd_SayCommand (slot 1)
   Also in vtable: Event_SlashCmd_SayOfficer (slot 1)
   Also in vtable: Event_SlashCmd_SayPlatoon (slot 1)
   Also in vtable: Event_SlashCmd_SayChannel (slot 1)
   Also in vtable: Event_SlashCmd_Tell (slot 1)
   Also in vtable: Event_SlashCmd_Petition (slot 1)
   Also in vtable: Event_SlashCmd_NTell (slot 1)
   Also in vtable: Event_SlashCmd_EquipItem (slot 1)
   Also in vtable: Event_SlashCmd_UnequipItem (slot 1)
   Also in vtable: Event_SlashCmd_UseItem (slot 1)
   Also in vtable: Event_SlashCmd_DeleteItem (slot 1)
   Also in vtable: Event_SlashCmd_GMDeleteItem (slot 1)
   Also in vtable: Event_SlashCmd_RepairItem (slot 1)
   Also in vtable: Event_SlashCmd_Respawn (slot 1)
   Also in vtable: Event_SlashCmd_TestSequence (slot 1)
   Also in vtable: Event_SlashCmd_MissionAssign (slot 1)
   Also in vtable: Event_SlashCmd_MissionClear (slot 1)
   Also in vtable: Event_SlashCmd_MissionClearActive (slot 1)
   Also in vtable: Event_SlashCmd_MissionClearHistory (slot 1)
   Also in vtable: Event_SlashCmd_MissionList (slot 1)
   Also in vtable: Event_SlashCmd_MissionListFull (slot 1)
   Also in vtable: Event_SlashCmd_MissionDetails (slot 1)
   Also in vtable: Event_SlashCmd_MissionAdvance (slot 1)
   Also in vtable: Event_SlashCmd_MissionReset (slot 1)
   Also in vtable: Event_SlashCmd_MissionComplete (slot 1)
   Also in vtable: Event_SlashCmd_MissionSetAvailable (slot 1)
   Also in vtable: Event_SlashCmd_GiveXp (slot 1)
   Also in vtable: Event_SlashCmd_GiveNaqahdah (slot 1)
   Also in vtable: Event_SlashCmd_GiveItem (slot 1)
   Also in vtable: Event_SlashCmd_GiveAmmo (slot 1)
   Also in vtable: Event_SlashCmd_GiveFaction (slot 1)
   Also in vtable: Event_SlashCmd_SetFaction (slot 1)
   Also in vtable: Event_SlashCmd_GiveAbility (slot 1)
   Also in vtable: Event_SlashCmd_InvokeAbility (slot 1)
   Also in vtable: Event_SlashCmd_GiveRespawner (slot 1)
   Also in vtable: Event_SlashCmd_SetTechSkill (slot 1)
   Also in vtable: Event_SlashCmd_GiveBlueprint (slot 1)
   Also in vtable: Event_SlashCmd_GiveGearset (slot 1)
   Also in vtable: Event_SlashCmd_GiveInventory (slot 1)
   Also in vtable: Event_SlashCmd_Goto (slot 1)
   Also in vtable: Event_SlashCmd_GotoLocation (slot 1)
   Also in vtable: Event_SlashCmd_GotoXYZ (slot 1)
   Also in vtable: Event_SlashCmd_ReloadOrganizations (slot 1)
   Also in vtable: Event_SlashCmd_ReloadInventory (slot 1)
   Also in vtable: Event_SlashCmd_Users (slot 1)
   Also in vtable: Event_SlashCmd_SetHideGM (slot 1)
   Also in vtable: Event_SlashCmd_PrintStats (slot 1)
   Also in vtable: Event_SlashCmd_SetGodMode (slot 1)
   Also in vtable: Event_SlashCmd_SetNoXP (slot 1)
   Also in vtable: Event_SlashCmd_SetNoDamageTimedMode (slot 1)
   Also in vtable: Event_SlashCmd_SetNoAggro (slot 1)
   Also in vtable: Event_SlashCmd_SetInfiniteAmmo (slot 1)
   Also in vtable: Event_SlashCmd_SetInvulnerable (slot 1)
   Also in vtable: Event_SlashCmd_SetNoXp (slot 1)
   Also in vtable: Event_SlashCmd_SetOmnipotent (slot 1)
   Also in vtable: Event_SlashCmd_SetIgnoreFocus (slot 1)
   Also in vtable: Event_SlashCmd_SetIgnoreHealth (slot 1)
   Also in vtable: Event_SlashCmd_SetFly (slot 1)
   Also in vtable: Event_SlashCmd_SetGhost (slot 1)
   Also in vtable: Event_SlashCmd_SetSpectator (slot 1)
   Also in vtable: Event_SlashCmd_SetNoTarget (slot 1)
   Also in vtable: Event_SlashCmd_SetPVP (slot 1)
   Also in vtable: Event_SlashCmd_SetSpeed (slot 1)
   Also in vtable: Event_SlashCmd_SetArchetype (slot 1)
   Also in vtable: Event_SlashCmd_SetHealth (slot 1)
   Also in vtable: Event_SlashCmd_SetHealthMax (slot 1)
   Also in vtable: Event_SlashCmd_SetFocus (slot 1)
   Also in vtable: Event_SlashCmd_SetFocusMax (slot 1)
   Also in vtable: Event_SlashCmd_SetFlag (slot 1)
   Also in vtable: Event_SlashCmd_SetMobStance (slot 1)
   Also in vtable: Event_SlashCmd_SetTarget (slot 1)
   Also in vtable: Event_SlashCmd_SetLevel (slot 1)
   Also in vtable: Event_SlashCmd_ResetAbilities (slot 1)
   Also in vtable: Event_SlashCmd_GiveAllAbilities (slot 1)
   Also in vtable: Event_SlashCmd_Respec (slot 1)
   Also in vtable: Event_SlashCmd_RespecAbility (slot 1)
   Also in vtable: Event_SlashCmd_RespecCraft (slot 1)
   Also in vtable: Event_SlashCmd_Kill (slot 1)
   Also in vtable: Event_SlashCmd_Spawn (slot 1)
   Also in vtable: Event_SlashCmd_Summon (slot 1)
   Also in vtable: Event_SlashCmd_ShowLog (slot 1)
   Also in vtable: Event_SlashCmd_ShowMobCount (slot 1)
   Also in vtable: Event_SlashCmd_ShowFPS (slot 1)
   Also in vtable: Event_SlashCmd_ShowIP (slot 1)
   Also in vtable: Event_SlashCmd_ShowInventory (slot 1)
   Also in vtable: Event_SlashCmd_ShowPlayer (slot 1)
   Also in vtable: Event_SlashCmd_ShowTriggers (slot 1)
   Also in vtable: Event_SlashCmd_ShowSpawns (slot 1)
   Also in vtable: Event_SlashCmd_ShowWaypoints (slot 1)
   Also in vtable: Event_SlashCmd_ShowNavMesh (slot 1)
   Also in vtable: Event_SlashCmd_ShowMobPaths (slot 1)
   Also in vtable: Event_SlashCmd_ShowTargetLocation (slot 1)
   Also in vtable: Event_SlashCmd_ShowPosition (slot 1)
   Also in vtable: Event_SlashCmd_ShowRotation (slot 1)
   Also in vtable: Event_SlashCmd_ShowMemory (slot 1)
   Also in vtable: Event_SlashCmd_Location (slot 1)
   Also in vtable: Event_SlashCmd_MobData (slot 1)
   Also in vtable: Event_SlashCmd_DebugTarget (slot 1)
   Also in vtable: Event_SlashCmd_DebugStartMinigame (slot 1)
   Also in vtable: Event_SlashCmd_DebugMinigameInstance (slot 1)
   Also in vtable: Event_SlashCmd_DebugSpectateMinigame (slot 1)
   Also in vtable: Event_SlashCmd_DebugJoinMinigame (slot 1)
   Also in vtable: Event_SlashCmd_MinigameComplete (slot 1)
   Also in vtable: Event_SlashCmd_GiveMinigameContact (slot 1)
   Also in vtable: Event_SlashCmd_RemoveMinigameContact (slot 1)
   Also in vtable: Event_SlashCmd_StartMinigame (slot 1)
   Also in vtable: Event_SlashCmd_DebugFlash (slot 1)
   Also in vtable: Event_SlashCmd_Exit (slot 1)
   Also in vtable: Event_SlashCmd_DHD (slot 1)
   Also in vtable: Event_SlashCmd_ListItems (slot 1)
   Also in vtable: Event_SlashCmd_ActivateBandolierSlot (slot 1)
   Also in vtable: Event_SlashCmd_TimeofDay (slot 1)
   Also in vtable: Event_SlashCmd_Interact (slot 1)
   Also in vtable: Event_SlashCmd_DebugAbilityOnMob (slot 1)
   Also in vtable: Event_SlashCmd_DebugBehaviorsOnMob (slot 1)
   Also in vtable: Event_SlashCmd_DebugPathsOnMob (slot 1)
   Also in vtable: Event_SlashCmd_DebugEvents (slot 1)
   Also in vtable: Event_SlashCmd_DebugPerformance (slot 1)
   Also in vtable: Event_SlashCmd_InitialResponse (slot 1)
   Also in vtable: Event_SlashCmd_DialogButtonChoice (slot 1)
   Also in vtable: Event_SlashCmd_ShowDialog (slot 1)
   Also in vtable: Event_SlashCmd_TrainAbility (slot 1)
   Also in vtable: Event_SlashCmd_PurchaseItem (slot 1)
   Also in vtable: Event_SlashCmd_LogOff (slot 1)
   Also in vtable: Event_SlashCmd_Despawn (slot 1)
   Also in vtable: Event_SlashCmd_RechargeItem (slot 1)
   Also in vtable: Event_SlashCmd_SetMobAttribute (slot 1)
   Also in vtable: Event_SlashCmd_GetMobAttribute (slot 1)
   Also in vtable: Event_SlashCmd_LootItem (slot 1)
   Also in vtable: Event_SlashCmd_ListAbilities (slot 1)
   Also in vtable: Event_SlashCmd_AbandonMission (slot 1)
   Also in vtable: Event_SlashCmd_ShareMission (slot 1)
   Also in vtable: Event_SlashCmd_ShareMissionAccept (slot 1)
   Also in vtable: Event_SlashCmd_ShareMissionDecline (slot 1)
   Also in vtable: Event_SlashCmd_MissionAbandon (slot 1)
   Also in vtable: Event_SlashCmd_CombatDebug (slot 1)
   Also in vtable: Event_SlashCmd_CombatDebugVerbose (slot 1)
   Also in vtable: Event_SlashCmd_HealDebug (slot 1)
   Also in vtable: Event_SlashCmd_AbilityDebug (slot 1)
   Also in vtable: Event_SlashCmd_MoveItem (slot 1)
   Also in vtable: Event_SlashCmd_GetItemInfo (slot 1)
   Also in vtable: Event_SlashCmd_ChangeAmmo (slot 1)
   Also in vtable: Event_SlashCmd_ChatJoin (slot 1)
   Also in vtable: Event_SlashCmd_ChatLeave (slot 1)
   Also in vtable: Event_SlashCmd_ChatSetAFKMessage (slot 1)
   Also in vtable: Event_SlashCmd_ChatSetDNDMessage (slot 1)
   Also in vtable: Event_SlashCmd_ChatIgnore (slot 1)
   Also in vtable: Event_SlashCmd_ChatFriend (slot 1)
   Also in vtable: Event_SlashCmd_ChatUnfriend (slot 1)
   Also in vtable: Event_SlashCmd_ChatList (slot 1)
   Also in vtable: Event_SlashCmd_ChatMute (slot 1)
   Also in vtable: Event_SlashCmd_ChatUnmute (slot 1)
   Also in vtable: Event_SlashCmd_ChatKick (slot 1)
   Also in vtable: Event_SlashCmd_ChatOp (slot 1)
   Also in vtable: Event_SlashCmd_ChatBan (slot 1)
   Also in vtable: Event_SlashCmd_ChatUnban (slot 1)
   Also in vtable: Event_SlashCmd_ChatPassword (slot 1)
   Also in vtable: Event_SlashCmd_SquadInvite (slot 1)
   Also in vtable: Event_SlashCmd_SquadInviteAccept (slot 1)
   Also in vtable: Event_SlashCmd_SquadInviteDecline (slot 1)
   Also in vtable: Event_SlashCmd_SquadKick (slot 1)
   Also in vtable: Event_SlashCmd_SquadPromote (slot 1)
   Also in vtable: Event_SlashCmd_SquadLeave (slot 1)
   Also in vtable: Event_SlashCmd_CommandInvite (slot 1)
   Also in vtable: Event_SlashCmd_CommandInviteAccept (slot 1)
   Also in vtable: Event_SlashCmd_CommandInviteDecline (slot 1)
   Also in vtable: Event_SlashCmd_CommandLeave (slot 1)
   Also in vtable: Event_SlashCmd_CommandKick (slot 1)
   Also in vtable: Event_SlashCmd_CommandPromote (slot 1)
   Also in vtable: Event_SlashCmd_CommandDemote (slot 1)
   Also in vtable: Event_SlashCmd_CommandMOTD (slot 1)
   Also in vtable: Event_SlashCmd_SetCommandNote (slot 1)
   Also in vtable: Event_SlashCmd_SetCommandOfficerNote (slot 1)
   Also in vtable: Event_SlashCmd_TeamInvite (slot 1)
   Also in vtable: Event_SlashCmd_TeamInviteAccept (slot 1)
   Also in vtable: Event_SlashCmd_TeamInviteDecline (slot 1)
   Also in vtable: Event_SlashCmd_TeamLeave (slot 1)
   Also in vtable: Event_SlashCmd_TeamKick (slot 1)
   Also in vtable: Event_SlashCmd_TeamPromote (slot 1)
   Also in vtable: Event_SlashCmd_TeamDemote (slot 1)
   Also in vtable: Event_SlashCmd_TeamMOTD (slot 1)
   Also in vtable: Event_SlashCmd_SetTeamNote (slot 1)
   Also in vtable: Event_SlashCmd_SetTeamOfficerNote (slot 1)
   Also in vtable: Event_SlashCmd_ChooseOrgName (slot 1)
   Also in vtable: Event_SlashCmd_Who (slot 1)
   Also in vtable: Event_SlashCmd_toggleAutoCycleAbility (slot 1)
   Also in vtable: Event_SlashCmd_Walk (slot 1)
   Also in vtable: Event_SlashCmd_Run (slot 1)
   Also in vtable: Event_SlashCmd_ShowCover (slot 1)
   Also in vtable: Event_SlashCmd_ShowCommandWaypoints (slot 1)
   Also in vtable: Event_SlashCmd_ShowSpawnSet (slot 1)
   Also in vtable: Event_SlashCmd_ShowArea (slot 1)
   Also in vtable: Event_SlashCmd_ShowRegion (slot 1)
   Also in vtable: Event_SlashCmd_XRayEyes (slot 1)
   Also in vtable: Event_SlashCmd_Invisible (slot 1)
   Also in vtable: Event_SlashCmd_Physics (slot 1)
   Also in vtable: Event_SlashCmd_GiveStargateAddress (slot 1)
   Also in vtable: Event_SlashCmd_RemoveStargateAddress (slot 1)
   Also in vtable: Event_SlashCmd_GMChat (slot 1)
   Also in vtable: Event_SlashCmd_GMShout (slot 1)
   Also in vtable: Event_SlashCmd_ToggleCombatLOS (slot 1)
   Also in vtable: Event_SlashCmd_TestLOS (slot 1)
   Also in vtable: Event_SlashCmd_RegenerateCoverLinks (slot 1)
   Also in vtable: Event_SlashCmd_ChangeCoverWeight (slot 1)
   Also in vtable: Event_SlashCmd_ChangeCoverStanceWeight (slot 1)
   Also in vtable: Event_SlashCmd_LoadConstants (slot 1)
   Also in vtable: Event_SlashCmd_LoadAbility (slot 1)
   Also in vtable: Event_SlashCmd_LoadNACSI (slot 1)
   Also in vtable: Event_SlashCmd_LoadAbilitySet (slot 1)
   Also in vtable: Event_SlashCmd_LoadBehavior (slot 1)
   Also in vtable: Event_SlashCmd_LoadMOB (slot 1)
   Also in vtable: Event_SlashCmd_LoadInteractionSet (slot 1)
   Also in vtable: Event_SlashCmd_LoadItem (slot 1)
   Also in vtable: Event_SlashCmd_LoadMission (slot 1)
   Also in vtable: Event_SlashCmd_SetMobVariable (slot 1)
   Also in vtable: Event_SlashCmd_PetInvokeAbility (slot 1)
   Also in vtable: Event_SlashCmd_PetInvokeCommand (slot 1)
   Also in vtable: Event_SlashCmd_PetAbilityToggle (slot 1)
   Also in vtable: Event_SlashCmd_DebugInteract (slot 1)
   Also in vtable: Event_SlashCmd_ShowFlag (slot 1)
   Also in vtable: Event_SlashCmd_ListInteractions (slot 1)
   Also in vtable: Event_SlashCmd_SetMobAbilitySet (slot 1)
   Also in vtable: Event_SlashCmd_GiveTrainingPoints (slot 1)
   Also in vtable: Event_SlashCmd_EmitBehaviorEventOnMob (slot 1)
   Also in vtable: Event_SlashCmd_AddBehaviorEventSet (slot 1)
   Also in vtable: Event_SlashCmd_RemoveBehaviorEventSet (slot 1)
   Also in vtable: Event_SlashCmd_ForceClientCrash (slot 1)
   Also in vtable: Event_SlashCmd_ForceRenderThreadCrash (slot 1)
   Also in vtable: Event_SlashCmd_SetRingTransporterDestination (slot 1)
   Also in vtable: Event_SlashCmd_WorldInstanceReset (slot 1)
   Also in vtable: Event_SlashCmd_Unstuck (slot 1)
   Also in vtable: Event_SlashCmd_GiveExpertise (slot 1)
   Also in vtable: Event_SlashCmd_GiveAppliedSciencePoints (slot 1)
   Also in vtable: Event_SlashCmd_GiveRacialParadigmLevels (slot 1)
   Also in vtable: Event_SlashCmd_DumpObjects (slot 1)
   Also in vtable: Event_SlashCmd_DuelChallenge (slot 1)
   Also in vtable: Event_SlashCmd_DuelResponse (slot 1)
   Also in vtable: Event_SlashCmd_DuelForfeit (slot 1)
   Also in vtable: Event_SlashCmd_EnterErrorAIState (slot 1)
   Also in vtable: Event_SlashCmd_ExitErrorAIState (slot 1)
   Also in vtable: Event_SlashCmd_SpaceQueueStatus (slot 1)
   Also in vtable: Event_SlashCmd_SpaceQueuedResponse (slot 1)
   Also in vtable: Event_SlashCmd_SpaceQueueReadyResponse (slot 1)
   Also in vtable: Event_SlashCmd_PerfStatsByChannel (slot 1)
   Also in vtable: Event_SlashCmd_ShowInstanceFlag (slot 1)
   Also in vtable: Event_SlashCmd_SetInstanceFlag (slot 1)
   Also in vtable: Event_SlashCmd_ShowNavigation (slot 1)
   Also in vtable: Event_SlashCmd_ConfirmEffect (slot 1)
   Also in vtable: CME_TEXT_CMD_TOO_FEW_PARAMS (slot 1) */

char * CME_GenericEvent__vfunc_1(void)

{
  return "GenericEvent";
}


/* Also in vtable: CME_X___SubjectEvent (slot 0) */

undefined4 * __thiscall CME_X___SubjectEvent__vfunc_0(void *this,byte param_1)

{
  FUN_00441340(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CXMLParserRoot.c ========== */

/*
 * SGW.exe - CXMLParserRoot (2 functions)
 */


/* Library Function - Single Match
    public: __thiscall CXMLParserRoot::CXMLParserRoot(void)
   
   Library: Visual Studio 2010 Debug */

CXMLParserRoot * __thiscall CXMLParserRoot::CXMLParserRoot(CXMLParserRoot *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_017844e8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_013df370((undefined4 *)this);
  local_8 = 0;
  *(undefined ***)this = GFxSetBackgroundColor::vftable;
  FBSPSurfaceStaticLighting__vfunc_0(this + 4);
  ExceptionList = local_10;
  return this;
}




/* Library Function - Single Match
    public: __thiscall CXMLParserRoot::CXMLParserRoot(void)
   
   Library: Visual Studio 2010 Debug */

CXMLParserRoot * __thiscall CXMLParserRoot::CXMLParserRoot(CXMLParserRoot *this)

{
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_017844e8;
  local_10 = ExceptionList;
  ExceptionList = &local_10;
  FUN_013df370((undefined4 *)this);
  local_8 = 0;
  *(undefined ***)this = GASDoAction::vftable;
  FUN_01441890((undefined4 *)(this + 4));
  ExceptionList = local_10;
  return this;
}





/* ========== CoverInfo.c ========== */

/*
 * SGW.exe - CoverInfo (1 functions)
 */

undefined4 * __thiscall CoverInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00e71c30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CoverNodeUserCacheData.c ========== */

/*
 * SGW.exe - CoverNodeUserCacheData (1 functions)
 */

/* Also in vtable: CoverNodeUserCacheData (slot 0) */

undefined4 * __thiscall CoverNodeUserCacheData__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016c2e43;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  FUN_00905c30((int *)((int)this + 0xc));
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== DataType.c ========== */

/*
 * SGW.exe - DataType (1 functions)
 */

undefined4 * __thiscall DataType__vfunc_0(void *this,byte param_1)

{
  FUN_01595b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Dialog.c ========== */

/*
 * SGW.exe - Dialog (1 functions)
 */

undefined4 * __thiscall Dialog__vfunc_0(void *this,byte param_1)

{
  FUN_00d23ef0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DialogController_AvailableDialog.c ========== */

/*
 * SGW.exe - DialogController_AvailableDialog (1 functions)
 */

undefined4 * __thiscall DialogController_AvailableDialog__vfunc_0(void *this,byte param_1)

{
  FUN_00d272c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DialogGiverEntry.c ========== */

/*
 * SGW.exe - DialogGiverEntry (1 functions)
 */

void * __thiscall DialogGiverEntry__vfunc_0(void *this,void *param_1)

{
  void *pvVar1;
  undefined4 uVar2;
  int iVar3;
  uint uVar4;
  char cVar5;
  undefined4 uVar6;
  TypeDescriptor *pTVar7;
  TypeDescriptor *pTVar8;
  undefined4 uVar9;
  wchar_t awStack_2c [2];
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01711621;
  pvStack_c = ExceptionList;
  uVar9 = 0;
  pTVar8 = &GameBeing::RTTI_Type_Descriptor;
  pTVar7 = &GameEntityBase::RTTI_Type_Descriptor;
  uVar6 = 0;
  uVar2 = *(undefined4 *)((int)this + 0x24);
  cVar5 = '\0';
  ExceptionList = &pvStack_c;
  pvVar1 = (void *)FUN_00c66ad0();
  uVar2 = FUN_00dd0de0(pvVar1,uVar2,cVar5);
  iVar3 = __RTDynamicCast(uVar2,uVar6,pTVar7,pTVar8,uVar9);
  if (iVar3 == 0) {
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    awStack_2c[0] = L'\0';
    awStack_2c[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),awStack_2c);
    uVar4 = std::char_traits<wchar_t>::length(L"UNKNOWN MISSION GIVER ENTITY ID");
    FUN_00438520(param_1,L"UNKNOWN MISSION GIVER ENTITY ID",uVar4);
    ExceptionList = pvStack_c;
    return param_1;
  }
  pvVar1 = FUN_00e9f810(auStack_28,iVar3);
  uStack_4 = 1;
  FUN_0047b670(param_1,(void *)(iVar3 + 0x10c),(int)pvVar1);
  uStack_4 = uStack_4 & 0xffffff00;
  if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 7;
  awStack_2c[0] = L'\0';
  awStack_2c[1] = L'\0';
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,awStack_2c);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== EntityAppearanceLog.c ========== */

/*
 * SGW.exe - EntityAppearanceLog (1 functions)
 */

void * __thiscall EntityAppearanceLog__vfunc_0(void *this,byte param_1)

{
  GameEntity__unknown_00cff140(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FGlobalDataStoreClientManager.c ========== */

/*
 * SGW.exe - FGlobalDataStoreClientManager (1 functions)
 */

/* Also in vtable: FGlobalDataStoreClientManager (slot 0) */

undefined4 * __thiscall FGlobalDataStoreClientManager__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FGlobalDataStoreClientManager::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== G_00_07.c ========== */

/*
 * SGW.exe - G_00_07 (2 functions)
 */

/* [VTable] G_00_07___TD3DResourceArray virtual function #1
   VTable: vtable_G_00_07___TD3DResourceArray at 0184e728 */

int __fastcall G_00_07___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) * 2;
}


/* Also in vtable: 00_07__00_U__TStaticMeshFullVertex___TD3DResourceArray (slot 0)
   Also in vtable: 00_07__01_U__TStaticMeshFullVertex___TD3DResourceArray (slot 0)
   Also in vtable: 00_07__02_U__TStaticMeshFullVertex___TD3DResourceArray (slot 0)
   Also in vtable: 00_07__03_U__TStaticMeshFullVertex___TD3DResourceArray (slot 0)
   Also in vtable: 00_07_UFShadowVertex___TD3DResourceArray (slot 0)
   Also in vtable: 00_07_UFPackedNormal___TD3DResourceArray (slot 0)
   Also in vtable: 07__0A_UFVector2D___TD3DResourceArray (slot 0)
   Also in vtable: G_00_07___TD3DResourceArray (slot 0)
   Also in vtable: 00_07_UFSoftSkinVertex___TD3DResourceArray (slot 0)
   Also in vtable: FNULLSceneInterface (slot 20)
   Also in vtable: FScene (slot 20)
   Also in vtable: 07_M_0A___TD3DResourceArray (slot 0)
   Also in vtable: FCanvasScene (slot 21)
   Also in vtable: 07__0A_UFModelVertex___TD3DResourceArray (slot 0)
   Also in vtable: FDebuggerState (slot 1)
   Also in vtable: DSIdleState (slot 1)
   Also in vtable: DSWaitForInput (slot 1)
   Also in vtable: DSWaitForCondition (slot 1)
   Also in vtable: DSRunToCursor (slot 1)
   Also in vtable: DSStepOut (slot 1)
   Also in vtable: DSStepInto (slot 1)
   Also in vtable: DSStepOverStack (slot 1) */

undefined4 __fastcall _00_07__00_U__TStaticMeshFullVertex___TD3DResourceArray__vfunc_0(int param_1)

{
  return *(undefined4 *)(param_1 + 4);
}




/* ========== ICompositingProcessContinuation.c ========== */

/*
 * SGW.exe - ICompositingProcessContinuation (1 functions)
 */

undefined4 * __thiscall ICompositingProcessContinuation__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = ICompositingProcessContinuation::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ISGWMessage.c ========== */

/*
 * SGW.exe - ISGWMessage (1 functions)
 */

undefined4 * __thiscall ISGWMessage__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = ISGWMessage::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== IStream.c ========== */

/*
 * SGW.exe - IStream (1 functions)
 */


/* Library Function - Single Match
    public: __thiscall IStream::IStream(void)
   
   Libraries: Visual Studio 2005 Debug, Visual Studio 2008 Debug */

IStream * __thiscall IStream::IStream(IStream *this)

{
  FUN_01428d90((GRefCountBaseImpl *)this);
  *(undefined ***)this = GFxKeyboardState::IListener::vftable;
  return this;
}





/* ========== IUnknown.c ========== */

/*
 * SGW.exe - IUnknown (1 functions)
 */


/* Library Function - Single Match
    public: __thiscall IUnknown::IUnknown(void)
   
   Libraries: Visual Studio 2003 Debug, Visual Studio 2005 Debug, Visual Studio 2008 Debug, Visual
   Studio 2010 Debug */

IUnknown * __thiscall IUnknown::IUnknown(IUnknown *this)

{
  return this;
}





/* ========== ItemNameEntry.c ========== */

/*
 * SGW.exe - ItemNameEntry (1 functions)
 */

void * ItemNameEntry__vfunc_0(void *param_1,void *param_2)

{
  uint uVar1;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d9379;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  FUN_00e9f480(param_2,L" id=");
  *(undefined4 *)((int)param_1 + 0x18) = 7;
  param_2 = (void *)0x0;
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),(wchar_t *)&param_2);
  uVar1 = std::char_traits<wchar_t>::length(L"UNKNOWN ITEM");
  FUN_00438520(param_1,L"UNKNOWN ITEM",uVar1);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== LoginReplyHandler.c ========== */

/*
 * SGW.exe - LoginReplyHandler (1 functions)
 */

undefined4 * __thiscall LoginReplyHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00dde560(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MVSystemOptionPolicyFloat.c ========== */

/*
 * SGW.exe - MVSystemOptionPolicyFloat (8 functions)
 */

/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #4
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

undefined4 __fastcall MVSystemOptionPolicyFloat___SystemOption__vfunc_4(int param_1)

{
  if (*(float *)(param_1 + 0xb8) != *(float *)(param_1 + 0xbc)) {
    return 1;
  }
  return 0;
}


/* Also in vtable: MVSystemOptionPolicyFloat___SystemOption (slot 0) */

undefined4 * __thiscall MVSystemOptionPolicyFloat___SystemOption__vfunc_0(void *this,byte param_1)

{
  FUN_005778b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #5
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

void __thiscall
MVSystemOptionPolicyFloat___SystemOption__vfunc_5(void *this,void *param_1,int *param_2)

{
  int *piVar1;
  char cVar2;
  undefined1 uVar3;
  int iVar4;
  
  piVar1 = param_2;
  HVSystemOptionPolicyEnum__unknown_00577b60(param_1,param_2,L"name",(int)this + 8);
  HVSystemOptionPolicyEnum__unknown_00577b60(param_1,piVar1,L"text",(int)this + 0x24);
  HVSystemOptionPolicyEnum__unknown_00577b60(param_1,piVar1,L"tooltip",(int)this + 0x40);
  param_2 = *(int **)((int)this + 0x5c);
  HVSystemOptionPolicyEnum__unknown_00577be0(param_1,piVar1,L"type",(int *)&param_2);
  param_2 = (int *)CONCAT31(param_2._1_3_,*(undefined1 *)((int)this + 0x62));
  NVSystemOptionPolicyBool__unknown_00577c60(param_1,piVar1,L"disabled",(byte *)&param_2);
  HVSystemOptionPolicyEnum__unknown_00577b60
            (param_1,piVar1,L"customChangeFunction",(int)this + 0x68);
  iVar4 = FUN_005757f0();
  cVar2 = FUN_00574430(iVar4);
  if (cVar2 == '\0') {
    if (*(char *)((int)this + 0x61) == '\0') {
      param_2 = *(int **)((int)this + 0xbc);
    }
    else {
      param_2 = *(int **)((int)this + 0xb8);
    }
  }
  else {
    param_2 = *(int **)((int)this + 0xb4);
  }
  FUN_00577ad0(param_1,piVar1,L"value",(float *)&param_2);
  FUN_00577ad0(param_1,piVar1,L"proposedValue",(float *)((int)this + 0xbc));
  uVar3 = (**(code **)(*(int *)this + 0x10))();
  param_2 = (int *)CONCAT31(param_2._1_3_,uVar3);
  NVSystemOptionPolicyBool__unknown_00577c60(param_1,piVar1,L"changePending",(byte *)&param_2);
  (**(code **)(*(int *)((int)this + 0xc0) + 4))(param_1,piVar1);
  return;
}


/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #6
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

void __thiscall MVSystemOptionPolicyFloat___SystemOption__vfunc_6(void *this,void *param_1)

{
  char cVar1;
  uint uVar2;
  int iVar3;
  undefined4 *puVar4;
  char local_c9;
  float fStack_c8;
  undefined1 auStack_c4 [4];
  undefined4 local_c0 [4];
  undefined4 local_b0;
  uint local_ac;
  undefined1 auStack_a8 [4];
  undefined4 auStack_a4 [4];
  undefined4 uStack_94;
  uint uStack_90;
  basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> abStack_8c [76];
  basic_ios<wchar_t,struct_std::char_traits<wchar_t>_> abStack_40 [52];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0168f66c;
  pvStack_c = ExceptionList;
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  uVar2 = std::char_traits<char>::length("name");
  FUN_004377d0(auStack_c4,"name",uVar2);
  uStack_4 = 0;
  FUN_0043d380(param_1,auStack_c4,(undefined4 *)((int)this + 8));
  uStack_4 = 0xffffffff;
  if (0xf < local_ac) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_c0[0]);
  }
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  Detail__unknown_0042e9c0(abStack_8c,2,1);
  uStack_4 = 1;
  iVar3 = FUN_005757f0();
  cVar1 = FUN_00574430(iVar3);
  if (cVar1 == '\0') {
    if (*(char *)((int)this + 0x61) == '\0') {
      fStack_c8 = *(float *)((int)this + 0xbc);
    }
    else {
      fStack_c8 = *(float *)((int)this + 0xb8);
    }
  }
  else {
    fStack_c8 = *(float *)((int)this + 0xb4);
  }
  std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<(abStack_8c,fStack_c8);
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  uVar2 = std::char_traits<char>::length("value");
  FUN_004377d0(auStack_c4,"value",uVar2);
  uStack_4._0_1_ = 2;
  puVar4 = Detail__unknown_00439cb0(abStack_8c,auStack_a8);
  uStack_4._0_1_ = 3;
  FUN_0043d380(param_1,auStack_c4,puVar4);
  uStack_4._0_1_ = 2;
  if (7 < uStack_90) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_a4[0]);
  }
  uStack_90 = 7;
  fStack_c8 = 0.0;
  uStack_94 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_a4,(wchar_t *)&fStack_c8);
  uStack_4 = CONCAT31(uStack_4._1_3_,1);
  if (0xf < local_ac) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_c0[0]);
  }
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  uStack_4 = 0xffffffff;
  FUN_0042ea60((int)abStack_40);
  std::basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>::
  ~basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>(abStack_40);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #2
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

void __fastcall MVSystemOptionPolicyFloat___SystemOption__vfunc_2(int param_1)

{
  if ((*(float *)(param_1 + 0xbc) != *(float *)(param_1 + 0xb8)) &&
     (*(float *)(param_1 + 0xbc) = *(float *)(param_1 + 0xb8), *(char *)(param_1 + 0x61) == '\0')) {
    MVSystemOptionPolicyFloat__unknown_00579040(param_1);
    return;
  }
  return;
}


/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #1
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

void __fastcall MVSystemOptionPolicyFloat___SystemOption__vfunc_1(int param_1)

{
  if ((*(float *)(param_1 + 0xb8) != *(float *)(param_1 + 0xbc)) &&
     (*(float *)(param_1 + 0xb8) = *(float *)(param_1 + 0xbc), *(char *)(param_1 + 0x61) != '\0')) {
    MVSystemOptionPolicyFloat__unknown_00579040(param_1);
    return;
  }
  return;
}


/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #3
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

void __fastcall MVSystemOptionPolicyFloat___SystemOption__vfunc_3(int param_1)

{
  float fVar1;
  float fVar2;
  
  fVar1 = *(float *)(param_1 + 0xb0);
  if (*(float *)(param_1 + 0xbc) != fVar1) {
    fVar2 = *(float *)(param_1 + 0xc4);
    if ((fVar2 <= fVar1) && (fVar2 = fVar1, *(float *)(param_1 + 200) <= fVar1)) {
      fVar2 = *(float *)(param_1 + 200);
    }
    *(float *)(param_1 + 0xbc) = fVar2;
    if (*(char *)(param_1 + 0x61) == '\0') {
      MVSystemOptionPolicyFloat__unknown_00579040(param_1);
      return;
    }
  }
  return;
}


/* [VTable] MVSystemOptionPolicyFloat___SystemOption virtual function #7
   VTable: vtable_MVSystemOptionPolicyFloat___SystemOption at 0183ff84 */

void __thiscall MVSystemOptionPolicyFloat___SystemOption__vfunc_7(void *this,int param_1)

{
  float fVar1;
  float10 fVar2;
  float fVar3;
  
  if (*(int *)((int)this + 100) <= *(int *)(param_1 + 0x18)) {
    fVar2 = (float10)(**(code **)(*(int *)((int)this + 0xc0) + 8))(param_1);
    fVar1 = (float)fVar2;
    if ((float10)*(float *)((int)this + 0xbc) != fVar2) {
      fVar3 = *(float *)((int)this + 0xc4);
      if ((fVar3 <= fVar1) && (fVar3 = fVar1, *(float *)((int)this + 200) <= fVar1)) {
        fVar3 = *(float *)((int)this + 200);
      }
      *(float *)((int)this + 0xbc) = fVar3;
      if (*(char *)((int)this + 0x61) == '\0') {
        MVSystemOptionPolicyFloat__unknown_00579040((int)this);
      }
    }
    (**(code **)(*(int *)this + 4))();
    return;
  }
  (**(code **)(*(int *)this + 0xc))();
  (**(code **)(*(int *)this + 4))();
  return;
}




/* ========== MapBlip.c ========== */

/*
 * SGW.exe - MapBlip (1 functions)
 */

undefined4 * __thiscall MapBlip__vfunc_0(void *this,byte param_1)

{
  FUN_00eb6130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Minimap.c ========== */

/*
 * SGW.exe - Minimap (1 functions)
 */

undefined4 * __thiscall Minimap__vfunc_0(void *this,byte param_1)

{
  FUN_00e2ac10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Mission.c ========== */

/*
 * SGW.exe - Mission (1 functions)
 */

undefined4 * __thiscall Mission__vfunc_0(void *this,byte param_1)

{
  FUN_00d17660(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MissionGiverEntry.c ========== */

/*
 * SGW.exe - MissionGiverEntry (1 functions)
 */

void * __thiscall MissionGiverEntry__vfunc_0(void *this,void *param_1)

{
  uint uVar1;
  int iVar2;
  void *this_00;
  undefined4 **ppuVar3;
  void **ppvVar4;
  undefined4 *local_14;
  void *local_10;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0171169f;
  local_c = ExceptionList;
  if (*(int *)((int)this + 0x24) == 0) {
    ExceptionList = &local_c;
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    local_14 = (undefined4 *)0x0;
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),(wchar_t *)&local_14);
    uVar1 = std::char_traits<wchar_t>::length(L"UNKNOWN MISSION GIVER");
    FUN_00438520(param_1,L"UNKNOWN MISSION GIVER",uVar1);
  }
  else {
    ExceptionList = &local_c;
    iVar2 = FUN_00c66ad0();
    iVar2 = FUN_00d16800(*(void **)(*(int *)(iVar2 + 0x8c) + 0x20),*(int *)((int)this + 0x24));
    if (iVar2 == 0) {
      FUN_004398f0(param_1,L"UNKNOWN MISSION");
    }
    else {
      local_10 = *(void **)(iVar2 + 0x60);
      ppvVar4 = &local_10;
      ppuVar3 = &local_14;
      this_00 = (void *)Detail__unknown_004786f0();
      FUN_00e195b0(this_00,(int *)ppuVar3,(int *)ppvVar4);
      local_4 = 1;
      if (local_14 == (undefined4 *)0x0) {
        FUN_004398f0(param_1,L"UNKNOWN MISSION GIVER NAME STRING ID");
      }
      else {
        CEGUI__unknown_00437bf0(param_1,local_14 + 0xe);
      }
      local_4 = local_4 & 0xffffff00;
      if (((local_14 != (undefined4 *)0x0) && (local_14[1] = local_14[1] + -1, (int)local_14[1] < 1)
          ) && (local_14 != (undefined4 *)0x0)) {
        (**(code **)*local_14)(1);
        ExceptionList = local_10;
        return param_1;
      }
    }
  }
  ExceptionList = local_c;
  return param_1;
}




/* ========== MobNameEntry.c ========== */

/*
 * SGW.exe - MobNameEntry (1 functions)
 */

void * MobNameEntry__vfunc_0(void *param_1,void *param_2)

{
  undefined4 uVar1;
  void *this;
  int iVar2;
  uint uVar3;
  char cVar4;
  undefined4 uVar5;
  TypeDescriptor *pTVar6;
  TypeDescriptor *pTVar7;
  undefined4 uVar8;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d9379;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  uVar1 = FUN_00e9f480(param_2,L" id=");
  uVar8 = 0;
  pTVar7 = &GameBeing::RTTI_Type_Descriptor;
  pTVar6 = &GameEntityBase::RTTI_Type_Descriptor;
  uVar5 = 0;
  cVar4 = '\0';
  this = (void *)FUN_00c66ad0();
  uVar1 = FUN_00dd0de0(this,uVar1,cVar4);
  iVar2 = __RTDynamicCast(uVar1,uVar5,pTVar6,pTVar7,uVar8);
  param_2 = (void *)0x0;
  *(undefined4 *)((int)param_1 + 0x18) = 7;
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  if (iVar2 == 0) {
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),(wchar_t *)&param_2);
    uVar3 = std::char_traits<wchar_t>::length(L"UNKNOWN MOB ID");
    FUN_00438520(param_1,L"UNKNOWN MOB ID",uVar3);
  }
  else {
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),(wchar_t *)&param_2);
    FUN_00437900(param_1,(void *)(iVar2 + 0x10c),0,0xffffffff);
  }
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== NVSystemOptionPolicyBool.c ========== */

/*
 * SGW.exe - NVSystemOptionPolicyBool (8 functions)
 */

/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #4
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

bool __fastcall NVSystemOptionPolicyBool___SystemOption__vfunc_4(int param_1)

{
  return *(char *)(param_1 + 0xb2) != *(char *)(param_1 + 0xb3);
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #8
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __thiscall NVSystemOptionPolicyBool___SystemOption__vfunc_8(void *this,int param_1)

{
  int iVar1;
  
  if (*(uint *)((int)this + 0x20) < 8) {
    iVar1 = (int)this + 0xc;
  }
  else {
    iVar1 = *(int *)((int)this + 0xc);
  }
  *(int *)(param_1 + 8) = iVar1;
  *(undefined4 *)(param_1 + 0x18) = *(undefined4 *)((int)this + 100);
  (**(code **)(*(int *)((int)this + 0xb4) + 0xc))(param_1,(int)this + 0xb2);
  return;
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #5
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __thiscall
NVSystemOptionPolicyBool___SystemOption__vfunc_5(void *this,void *param_1,int *param_2)

{
  int *piVar1;
  char cVar2;
  undefined1 uVar3;
  int iVar4;
  
  piVar1 = param_2;
  HVSystemOptionPolicyEnum__unknown_00577b60(param_1,param_2,L"name",(int)this + 8);
  HVSystemOptionPolicyEnum__unknown_00577b60(param_1,piVar1,L"text",(int)this + 0x24);
  HVSystemOptionPolicyEnum__unknown_00577b60(param_1,piVar1,L"tooltip",(int)this + 0x40);
  param_2 = *(int **)((int)this + 0x5c);
  HVSystemOptionPolicyEnum__unknown_00577be0(param_1,piVar1,L"type",(int *)&param_2);
  param_2 = (int *)CONCAT31(param_2._1_3_,*(undefined1 *)((int)this + 0x62));
  NVSystemOptionPolicyBool__unknown_00577c60(param_1,piVar1,L"disabled",(byte *)&param_2);
  HVSystemOptionPolicyEnum__unknown_00577b60
            (param_1,piVar1,L"customChangeFunction",(int)this + 0x68);
  iVar4 = FUN_005757f0();
  cVar2 = FUN_00574430(iVar4);
  if (cVar2 == '\0') {
    if (*(char *)((int)this + 0x61) == '\0') {
      uVar3 = *(undefined1 *)((int)this + 0xb3);
    }
    else {
      uVar3 = *(undefined1 *)((int)this + 0xb2);
    }
  }
  else {
    uVar3 = *(undefined1 *)((int)this + 0xb1);
  }
  param_2 = (int *)CONCAT31(param_2._1_3_,uVar3);
  NVSystemOptionPolicyBool__unknown_00577c60(param_1,piVar1,L"value",(byte *)&param_2);
  NVSystemOptionPolicyBool__unknown_00577c60
            (param_1,piVar1,L"proposedValue",(byte *)((int)this + 0xb3));
  uVar3 = (**(code **)(*(int *)this + 0x10))();
  param_2 = (int *)CONCAT31(param_2._1_3_,uVar3);
  NVSystemOptionPolicyBool__unknown_00577c60(param_1,piVar1,L"changePending",(byte *)&param_2);
  (**(code **)(*(int *)((int)this + 0xb4) + 4))(param_1,piVar1);
  return;
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #6
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __thiscall NVSystemOptionPolicyBool___SystemOption__vfunc_6(void *this,void *param_1)

{
  char cVar1;
  uint uVar2;
  int iVar3;
  undefined4 *puVar4;
  char local_c9;
  undefined4 uStack_c8;
  undefined1 auStack_c4 [4];
  undefined4 local_c0 [4];
  undefined4 local_b0;
  uint local_ac;
  undefined1 auStack_a8 [4];
  undefined4 auStack_a4 [4];
  undefined4 uStack_94;
  uint uStack_90;
  basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> abStack_8c [76];
  basic_ios<wchar_t,struct_std::char_traits<wchar_t>_> abStack_40 [52];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0168f66c;
  pvStack_c = ExceptionList;
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  uVar2 = std::char_traits<char>::length("name");
  FUN_004377d0(auStack_c4,"name",uVar2);
  uStack_4 = 0;
  FUN_0043d380(param_1,auStack_c4,(undefined4 *)((int)this + 8));
  uStack_4 = 0xffffffff;
  if (0xf < local_ac) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_c0[0]);
  }
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  Detail__unknown_0042e9c0(abStack_8c,2,1);
  uStack_4 = 1;
  iVar3 = FUN_005757f0();
  cVar1 = FUN_00574430(iVar3);
  if (cVar1 == '\0') {
    if (*(char *)((int)this + 0x61) == '\0') {
      uStack_c8 = CONCAT31(uStack_c8._1_3_,*(undefined1 *)((int)this + 0xb3));
    }
    else {
      uStack_c8 = CONCAT31(uStack_c8._1_3_,*(undefined1 *)((int)this + 0xb2));
    }
  }
  else {
    uStack_c8 = CONCAT31(uStack_c8._1_3_,*(undefined1 *)((int)this + 0xb1));
  }
  std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<
            (abStack_8c,SUB41(uStack_c8,0));
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  uVar2 = std::char_traits<char>::length("value");
  FUN_004377d0(auStack_c4,"value",uVar2);
  uStack_4._0_1_ = 2;
  puVar4 = Detail__unknown_00439cb0(abStack_8c,auStack_a8);
  uStack_4._0_1_ = 3;
  FUN_0043d380(param_1,auStack_c4,puVar4);
  uStack_4._0_1_ = 2;
  if (7 < uStack_90) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_a4[0]);
  }
  uStack_90 = 7;
  uStack_c8 = 0;
  uStack_94 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_a4,(wchar_t *)&uStack_c8);
  uStack_4 = CONCAT31(uStack_4._1_3_,1);
  if (0xf < local_ac) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_c0[0]);
  }
  local_ac = 0xf;
  local_c9 = '\0';
  local_b0 = 0;
  std::char_traits<char>::assign((char *)local_c0,&local_c9);
  uStack_4 = 0xffffffff;
  FUN_0042ea60((int)abStack_40);
  std::basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>::
  ~basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>(abStack_40);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #2
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __fastcall NVSystemOptionPolicyBool___SystemOption__vfunc_2(int param_1)

{
  if ((*(char *)(param_1 + 0xb3) != *(char *)(param_1 + 0xb2)) &&
     (*(char *)(param_1 + 0xb3) = *(char *)(param_1 + 0xb2), *(char *)(param_1 + 0x61) == '\0')) {
    NVSystemOptionPolicyBool__unknown_00578f20(param_1);
    return;
  }
  return;
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #1
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __fastcall NVSystemOptionPolicyBool___SystemOption__vfunc_1(int param_1)

{
  if ((*(char *)(param_1 + 0xb2) != *(char *)(param_1 + 0xb3)) &&
     (*(char *)(param_1 + 0xb2) = *(char *)(param_1 + 0xb3), *(char *)(param_1 + 0x61) != '\0')) {
    NVSystemOptionPolicyBool__unknown_00578f20(param_1);
    return;
  }
  return;
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #3
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __fastcall NVSystemOptionPolicyBool___SystemOption__vfunc_3(int param_1)

{
  if ((*(char *)(param_1 + 0xb3) != *(char *)(param_1 + 0xb0)) &&
     (*(char *)(param_1 + 0xb3) = *(char *)(param_1 + 0xb0), *(char *)(param_1 + 0x61) == '\0')) {
    NVSystemOptionPolicyBool__unknown_00578f20(param_1);
    return;
  }
  return;
}


/* [VTable] NVSystemOptionPolicyBool___SystemOption virtual function #7
   VTable: vtable__NVSystemOptionPolicyBool___SystemOption at 0183ff5c */

void __thiscall NVSystemOptionPolicyBool___SystemOption__vfunc_7(void *this,int param_1)

{
  char cVar1;
  
  if (*(int *)(param_1 + 0x18) < *(int *)((int)this + 100)) {
    (**(code **)(*(int *)this + 0xc))();
    (**(code **)(*(int *)this + 4))();
    return;
  }
  cVar1 = (**(code **)(*(int *)((int)this + 0xb4) + 8))(param_1);
  if ((*(char *)((int)this + 0xb3) != cVar1) &&
     (*(char *)((int)this + 0xb3) = cVar1, *(char *)((int)this + 0x61) == '\0')) {
    NVSystemOptionPolicyBool__unknown_00578f20((int)this);
  }
  (**(code **)(*(int *)this + 4))();
  return;
}




/* ========== Organization.c ========== */

/*
 * SGW.exe - Organization (1 functions)
 */

undefined4 * __thiscall Organization__vfunc_0(void *this,byte param_1)

{
  FUN_00e4c570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PawnAppearanceJob.c ========== */

/*
 * SGW.exe - PawnAppearanceJob (1 functions)
 */

undefined4 * __thiscall PawnAppearanceJob__vfunc_0(void *this,byte param_1)

{
  FUN_00eb77e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PetAbilityAction.c ========== */

/*
 * SGW.exe - PetAbilityAction (1 functions)
 */

undefined4 * __thiscall PetAbilityAction__vfunc_0(void *this,byte param_1)

{
  FUN_00e3eba0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PetCommandAction.c ========== */

/*
 * SGW.exe - PetCommandAction (1 functions)
 */

undefined4 * __thiscall PetCommandAction__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_0170b830;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = PetCommandAction::vftable;
  *(undefined ***)this = Action::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== PetStanceAction.c ========== */

/*
 * SGW.exe - PetStanceAction (1 functions)
 */

undefined4 * __thiscall PetStanceAction__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_0170b830;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = PetStanceAction::vftable;
  *(undefined ***)this = Action::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== ProcessStatsMessage.c ========== */

/*
 * SGW.exe - ProcessStatsMessage (1 functions)
 */

undefined4 * __thiscall ProcessStatsMessage__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01794398;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ProcessStatsMessage::vftable;
  local_4 = 0xffffffff;
  FUN_01577570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== SequenceDataType.c ========== */

/*
 * SGW.exe - SequenceDataType (1 functions)
 */

undefined4 * __thiscall SequenceDataType__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01795c63;
  local_c = ExceptionList;
  local_4 = 0;
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




/* ========== SequenceMetaDataType.c ========== */

/*
 * SGW.exe - SequenceMetaDataType (1 functions)
 */

undefined4 * __thiscall SequenceMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_015981d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== Squad.c ========== */

/*
 * SGW.exe - Squad (1 functions)
 */

undefined4 * __thiscall Squad__vfunc_0(void *this,byte param_1)

{
  FUN_00e5cc40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== TableEntry.c ========== */

/*
 * SGW.exe - TableEntry (1 functions)
 */

void * TableEntry__vfunc_0(void *param_1,void *param_2)

{
  uint uVar1;
  void *pvVar2;
  wchar_t local_30 [4];
  undefined1 auStack_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
  puStack_8 = &LAB_01710b50;
  pvStack_c = ExceptionList;
  local_4 = 0;
  local_30[2] = L'\0';
  local_30[3] = L'\0';
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 7;
  local_30[0] = L'\0';
  local_30[1] = L'\0';
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_30);
  uVar1 = std::char_traits<wchar_t>::length(L"Unimplemented token(");
  FUN_00438520(param_1,L"Unimplemented token(",uVar1);
  local_4 = 0;
  local_30[2] = L'\x01';
  local_30[3] = L'\0';
  pvVar2 = FUN_0043a100(auStack_28,param_2,L")");
  local_4 = 1;
  FUN_004347d0(param_1,(int)pvVar2,0,0xffffffff);
  local_4 = local_4 & 0xffffff00;
  if (7 < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 7;
  param_2 = (void *)0x0;
  uStack_14 = 0;
  std::char_traits<wchar_t>::assign((wchar_t *)auStack_24,(wchar_t *)&param_2);
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== TaskInvCountEntry.c ========== */

/*
 * SGW.exe - TaskInvCountEntry (1 functions)
 */

void * TaskInvCountEntry__vfunc_0(void *param_1,void *param_2)

{
  int iVar1;
  int iVar2;
  uint uVar3;
  wchar_t local_90 [2];
  basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_> local_8c [76];
  basic_ios<wchar_t,struct_std::char_traits<wchar_t>_> abStack_40 [52];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01710f6a;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar1 = FUN_00e9f480(param_2,L"-#");
  FUN_00e9f480(param_2,L" taskid=");
  iVar2 = FUN_00c66ad0();
  iVar2 = FUN_00d16790(*(int *)(*(int *)(iVar2 + 0x8c) + 0x20));
  if ((iVar2 == 0) || (iVar2 = *(int *)(iVar2 + 0xc), iVar2 < 0)) {
    *(undefined4 *)((int)param_1 + 0x18) = 7;
    local_90[0] = L'\0';
    local_90[1] = L'\0';
    *(undefined4 *)((int)param_1 + 0x14) = 0;
    std::char_traits<wchar_t>::assign((wchar_t *)((int)param_1 + 4),local_90);
    uVar3 = std::char_traits<wchar_t>::length(L"UNKNOWN TASK ID");
    FUN_00438520(param_1,L"UNKNOWN TASK ID",uVar3);
  }
  else {
    Detail__unknown_0042e9c0(local_8c,2,1);
    local_4 = 1;
    std::basic_ostream<wchar_t,struct_std::char_traits<wchar_t>_>::operator<<
              (local_8c,iVar1 - iVar2);
    Detail__unknown_00439cb0(local_8c,param_1);
    local_4 = local_4 & 0xffffff00;
    FUN_0042ea60((int)abStack_40);
    std::basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>::
    ~basic_ios<wchar_t,struct_std::char_traits<wchar_t>_>(abStack_40);
  }
  ExceptionList = pvStack_c;
  return param_1;
}




/* ========== Team.c ========== */

/*
 * SGW.exe - Team (1 functions)
 */

undefined4 * __thiscall Team__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_017122c8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = Team::vftable;
  local_4 = 0xffffffff;
  FUN_00e4c570(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== TerrainDecalTessellationIndexBufferType.c ========== */

/*
 * SGW.exe - TerrainDecalTessellationIndexBufferType (1 functions)
 */

/* [VTable] TerrainDecalTessellationIndexBufferType virtual function #7
   VTable: vtable_TerrainDecalTessellationIndexBufferType at 01903abc */

undefined4 * __thiscall TerrainDecalTessellationIndexBufferType__vfunc_7(void *this,byte param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016cc818;
  pvStack_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &pvStack_c;
  FUN_009b3cf0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvStack_c;
  return this;
}




/* ========== TerrainTessellationIndexBufferType.c ========== */

/*
 * SGW.exe - TerrainTessellationIndexBufferType (1 functions)
 */

/* [VTable] TerrainTessellationIndexBufferType virtual function #7
   VTable: vtable_TerrainTessellationIndexBufferType at 01903a94 */

undefined4 * __thiscall TerrainTessellationIndexBufferType__vfunc_7(void *this,byte param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016cc7f8;
  pvStack_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &pvStack_c;
  FUN_009b2eb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvStack_c;
  return this;
}




/* ========== VBlobDataType.c ========== */

/*
 * SGW.exe - VBlobDataType (1 functions)
 */

undefined4 * __thiscall VBlobDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_0159b270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VMailBoxDataType.c ========== */

/*
 * SGW.exe - VMailBoxDataType (1 functions)
 */

undefined4 * __thiscall VMailBoxDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_0159b480(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VPythonDataType.c ========== */

/*
 * SGW.exe - VPythonDataType (1 functions)
 */

undefined4 * __thiscall VPythonDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_0159a700(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VStringDataType.c ========== */

/*
 * SGW.exe - VStringDataType (1 functions)
 */

undefined4 * __thiscall VStringDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_0159a360(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VWideStringDataType.c ========== */

/*
 * SGW.exe - VWideStringDataType (1 functions)
 */

undefined4 * __thiscall VWideStringDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_0159a550(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VendorBuyItem.c ========== */

/*
 * SGW.exe - VendorBuyItem (1 functions)
 */

undefined4 * __thiscall VendorBuyItem__vfunc_0(void *this,byte param_1)

{
  FUN_00d21430(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== VendorInvItem.c ========== */

/*
 * SGW.exe - VendorInvItem (1 functions)
 */

undefined4 * __thiscall VendorInvItem__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = VendorInvItem::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== XMLHandle.c ========== */

/*
 * SGW.exe - XMLHandle (1 functions)
 */

undefined4 * __thiscall XMLHandle__vfunc_0(void *this,byte param_1)

{
  FUN_0155ccb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== XMLSection.c ========== */

/*
 * SGW.exe - XMLSection (37 functions)
 */

/* [VTable] XMLSection virtual function #1
   VTable: vtable_XMLSection at 017fd22c */

int __fastcall XMLSection__vfunc_1(int param_1)

{
  if (*(int *)(param_1 + 0x4c) == 0) {
    return 0;
  }
  return *(int *)(param_1 + 0x50) - *(int *)(param_1 + 0x4c) >> 2;
}


/* [VTable] XMLSection virtual function #12
   VTable: vtable_XMLSection at 017fd22c */

int __fastcall XMLSection__vfunc_12(int param_1)

{
  int *piVar1;
  int local_8 [2];
  
  piVar1 = local_8;
  if (*(int *)(param_1 + 0x5c) == 0) {
    local_8[0] = 0;
  }
  else {
    local_8[0] = *(int *)(*(int *)(param_1 + 0x5c) + 0xc);
  }
  local_8[1] = 0x400;
  if (local_8[0] < 0x401) {
    piVar1 = local_8 + 1;
  }
  return *piVar1;
}


/* [VTable] XMLSection virtual function #3
   VTable: vtable_XMLSection at 017fd22c */

int * __thiscall XMLSection__vfunc_3(void *this,int *param_1,uint param_2)

{
  int iVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167ce79;
  local_c = ExceptionList;
  iVar1 = *(int *)((int)this + 0x4c);
  if ((iVar1 != 0) && (param_2 < (uint)(*(int *)((int)this + 0x50) - iVar1 >> 2))) {
    if ((iVar1 == 0) ||
       (ExceptionList = &local_c, (uint)(*(int *)((int)this + 0x50) - iVar1 >> 2) <= param_2)) {
      ExceptionList = &local_c;
      _invalid_parameter_noinfo();
    }
    iVar1 = *(int *)(*(int *)((int)this + 0x4c) + param_2 * 4);
    *param_1 = iVar1;
    if (iVar1 != 0) {
      FUN_00457e40(iVar1);
    }
    ExceptionList = local_c;
    return param_1;
  }
  *param_1 = 0;
  return param_1;
}


/* WARNING: Removing unreachable block (ram,0x0046b88e) */
/* WARNING: Removing unreachable block (ram,0x0046b8d6) */
/* [VTable] XMLSection virtual function #7
   VTable: vtable_XMLSection at 017fd22c */

int * __thiscall XMLSection__vfunc_7(void *this,int *param_1,int param_2)

{
  int *piVar1;
  uint uVar2;
  int iVar3;
  char *pcVar4;
  uint uVar5;
  uint uVar6;
  char *pcVar7;
  code *pcVar8;
  int *piVar9;
  bool bVar10;
  char cStack_31;
  int local_30;
  undefined4 local_2c;
  undefined1 local_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  uint local_4;
  
  pcVar8 = _invalid_parameter_noinfo_exref;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167da61;
  local_c = ExceptionList;
  iVar3 = (int)this + 0x48;
  local_2c = 0;
  piVar9 = *(int **)((int)this + 0x4c);
  ExceptionList = &local_c;
  local_30 = iVar3;
  if (*(int **)((int)this + 0x50) < piVar9) {
    ExceptionList = &local_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    piVar1 = *(int **)(iVar3 + 8);
    if (piVar1 < *(int **)(iVar3 + 4)) {
      (*pcVar8)();
    }
    if (piVar9 == piVar1) {
      *param_1 = 0;
      ExceptionList = local_c;
      return param_1;
    }
    if (*(int **)(iVar3 + 8) <= piVar9) {
      (*pcVar8)();
    }
    iVar3 = (**(code **)(*(int *)*piVar9 + 0x2c))(local_28);
    local_4 = 1;
    uVar5 = *(uint *)(param_2 + 0x14);
    if (*(uint *)(param_2 + 0x18) < 0x10) {
      pcVar7 = (char *)(param_2 + 4);
    }
    else {
      pcVar7 = *(char **)(param_2 + 4);
    }
    uVar2 = *(uint *)(iVar3 + 0x14);
    uVar6 = uVar2;
    if (uVar5 <= uVar2) {
      uVar6 = uVar5;
    }
    if (*(uint *)(iVar3 + 0x18) < 0x10) {
      pcVar4 = (char *)(iVar3 + 4);
    }
    else {
      pcVar4 = *(char **)(iVar3 + 4);
    }
    iVar3 = std::char_traits<char>::compare(pcVar4,pcVar7,uVar6);
    bVar10 = false;
    if (iVar3 == 0) {
      if (uVar2 < uVar5) {
        uVar5 = 0xffffffff;
      }
      else {
        uVar5 = (uint)(uVar2 != uVar5);
      }
      bVar10 = uVar5 == 0;
    }
    local_4 = local_4 & 0xffffff00;
    if (0xf < uStack_10) break;
    uStack_10 = 0xf;
    cStack_31 = '\0';
    uStack_14 = 0;
    std::char_traits<char>::assign((char *)auStack_24,&cStack_31);
    if (bVar10) {
      if (*(int **)(local_30 + 8) <= piVar9) {
        _invalid_parameter_noinfo();
      }
      iVar3 = *piVar9;
      *param_1 = iVar3;
      if (iVar3 != 0) {
        FUN_00457e40(iVar3);
      }
      ExceptionList = local_c;
      return param_1;
    }
    if (*(int **)(local_30 + 8) <= piVar9) {
      _invalid_parameter_noinfo();
    }
    piVar9 = piVar9 + 1;
    pcVar8 = _invalid_parameter_noinfo_exref;
    iVar3 = local_30;
  }
                    /* WARNING: Subroutine does not return */
  scalable_free(auStack_24[0]);
}


/* [VTable] XMLSection virtual function #15
   VTable: vtable_XMLSection at 017fd22c */

void __thiscall XMLSection__vfunc_15(void *this,undefined4 *param_1)

{
  undefined4 *puVar1;
  int iVar2;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167bca8;
  local_c = ExceptionList;
  local_4 = 0;
  puVar1 = *(undefined4 **)((int)this + 0x58);
  ExceptionList = &local_c;
  if (puVar1 != param_1) {
    ExceptionList = &local_c;
    *(undefined4 **)((int)this + 0x58) = param_1;
    if (param_1 != (undefined4 *)0x0) {
      FUN_00457e40((int)param_1);
    }
    if (puVar1 != (undefined4 *)0x0) {
      iVar2 = FUN_00457e50((int)puVar1);
      if (iVar2 == 1) {
        (**(code **)*puVar1)(1);
      }
    }
  }
  local_4 = 0xffffffff;
  FUN_004585a0((int *)&param_1);
  ExceptionList = local_c;
  return;
}


/* [VTable] XMLSection virtual function #42
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_42(void *this,int param_1)

{
  undefined1 uVar1;
  int iVar2;
  undefined4 local_28 [4];
  undefined4 uStack_18;
  uint uStack_14;
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d5d28;
  pvStack_c = ExceptionList;
  if (*(uint *)(param_1 + 0x18) < 0x10) {
    iVar2 = param_1 + 4;
  }
  else {
    iVar2 = *(int *)(param_1 + 4);
  }
  ExceptionList = &pvStack_c;
  FUN_00a35960(local_28,iVar2,*(uint *)(param_1 + 0x14));
  local_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))(local_28);
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_14) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_28[0]);
  }
  uStack_14 = 0xf;
  uStack_18 = 0;
  std::char_traits<char>::assign((char *)local_28,&stack0x00000000);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #13
   VTable: vtable_XMLSection at 017fd22c */

uint __thiscall XMLSection__vfunc_13(void *this,void *param_1)

{
  int *piVar1;
  uint uVar2;
  void *unaff_ESI;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167d7c0;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  if (*(int *)((int)param_1 + 0x14) != 0) {
    ExceptionList = &local_c;
    param_1 = (void *)FUN_004693d0(param_1,(undefined4 *)((int)this + 0x58),
                                   (void *)((int)this + 0x10));
    if ((char)param_1 == '\0') goto LAB_0046bbd9;
  }
  piVar1 = *(int **)((int)this + 0x58);
  if (piVar1 != (int *)0x0) {
    FUN_00457e40((int)this);
    local_4 = 0xffffffff;
    uVar2 = (**(code **)(*piVar1 + 0x38))();
    ExceptionList = unaff_ESI;
    return uVar2;
  }
LAB_0046bbd9:
  ExceptionList = local_c;
  return (uint)param_1 & 0xffffff00;
}


/* [VTable] XMLSection virtual function #36
   VTable: vtable_XMLSection at 017fd22c */

undefined4 __thiscall XMLSection__vfunc_36(void *this,int param_1)

{
  void *pvVar1;
  undefined4 extraout_EAX;
  undefined4 local_28 [4];
  undefined4 uStack_18;
  uint uStack_14;
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d5d28;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  pvVar1 = FUN_0046bc40(local_28,param_1);
  local_4 = 0;
  (**(code **)(*(int *)this + 0x8c))(pvVar1);
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_14) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_28[0]);
  }
  uStack_14 = 0xf;
  uStack_18 = 0;
  std::char_traits<char>::assign((char *)local_28,&stack0x00000000);
  ExceptionList = pvStack_10;
  return CONCAT31((int3)((uint)extraout_EAX >> 8),1);
}


/* [VTable] XMLSection virtual function #11
   VTable: vtable_XMLSection at 017fd22c */

void * __thiscall XMLSection__vfunc_11(void *this,void *param_1)

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
  local_11[0] = '\0';
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  if (pcVar1 == (char *)0x0) {
    std::char_traits<char>::assign((char *)((int)param_1 + 4),local_11);
    FUN_00437710(param_1,(void *)((int)this + 0x10),0,0xffffffff);
  }
  else {
    std::char_traits<char>::assign((char *)((int)param_1 + 4),local_11);
    uVar2 = std::char_traits<char>::length(pcVar1);
    FUN_004377d0(param_1,pcVar1,uVar2);
  }
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] XMLSection virtual function #17
   VTable: vtable_XMLSection at 017fd22c
   [String discovery] References: "XMLSection::asBool" */

uint __fastcall XMLSection__vfunc_17(int *param_1)

{
  uint uVar1;
  char *_Str1;
  int iVar2;
  undefined4 extraout_EAX;
  undefined4 uVar3;
  char *unaff_EBX;
  char local_45;
  undefined1 auStack_44 [8];
  undefined4 uStack_3c;
  uint uStack_38;
  undefined1 auStack_34 [4];
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  void *pvStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167cf78;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  local_45 = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,&local_45);
  uVar1 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar1);
  uStack_4 = 0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,1);
  local_10 = CONCAT31(local_10._1_3_,2);
  if (0xf < pvStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  pvStack_1c = (void *)0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffaf);
  _Str1 = unaff_EBX;
  if (uStack_38 < 0x10) {
    _Str1 = &stack0xffffffb4;
  }
  iVar2 = _stricmp(_Str1,"true");
  if (iVar2 == 0) {
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(unaff_EBX);
    }
    uStack_38 = 0xf;
    puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
    uStack_3c = 0;
    std::char_traits<char>::assign(&stack0xffffffb4,(char *)&puStack_8);
    ExceptionList = pvStack_18;
    return CONCAT31((int3)((uint)extraout_EAX >> 8),1);
  }
  if (uStack_38 < 0x10) {
    unaff_EBX = &stack0xffffffb4;
  }
  iVar2 = _stricmp(unaff_EBX,"false");
  if (iVar2 == 0) {
    local_10 = 0xffffffff;
    uVar1 = ZipFileSystem__unknown_00433ed0((uint)&stack0xffffffb0);
    ExceptionList = pvStack_18;
    return uVar1 & 0xffffff00;
  }
  (**(code **)(*param_1 + 0x2c))(auStack_34);
  local_14._0_1_ = 3;
  _00___FRawStaticIndexBuffer__vfunc_0();
  local_14 = CONCAT31(local_14._1_3_,2);
  ZipFileSystem__unknown_00433ed0((uint)&uStack_38);
  local_14 = 0xffffffff;
  uVar3 = ZipFileSystem__unknown_00433ed0((uint)&stack0xffffffac);
  ExceptionList = pvStack_1c;
  return CONCAT31((int3)((uint)uVar3 >> 8),pvStack_c._0_1_);
}


/* [VTable] XMLSection virtual function #18
   VTable: vtable_XMLSection at 017fd22c */

undefined1 * __fastcall XMLSection__vfunc_18(int *param_1)

{
  uint uVar1;
  char *_Str;
  undefined1 *puVar2;
  char *unaff_EBX;
  char local_45;
  undefined1 auStack_44 [8];
  uint uStack_3c;
  uint uStack_38;
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167cf50;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  local_45 = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,&local_45);
  uVar1 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar1);
  uStack_4 = 0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,0);
  local_10 = CONCAT31(local_10._1_3_,2);
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffaf);
  uVar1 = std::char_traits<char>::length("");
  uVar1 = FUN_004242c0(&stack0xffffffb0,0,uStack_3c,"",uVar1);
  if (uVar1 == 0) {
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(unaff_EBX);
    }
    uStack_38 = 0xf;
    uStack_3c = 0;
    std::char_traits<char>::assign(&stack0xffffffb4,&stack0xffffffaf);
  }
  else {
    _Str = unaff_EBX;
    if (uStack_38 < 0x10) {
      _Str = &stack0xffffffb4;
    }
    puVar2 = (undefined1 *)atoi(_Str);
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(unaff_EBX);
    }
    uStack_38 = 0xf;
    puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
    uStack_3c = 0;
    std::char_traits<char>::assign(&stack0xffffffb4,(char *)&puStack_8);
    puStack_8 = puVar2;
  }
  ExceptionList = pvStack_18;
  return puStack_8;
}


/* [VTable] XMLSection virtual function #19
   VTable: vtable_XMLSection at 017fd22c */

undefined1 * __fastcall XMLSection__vfunc_19(int *param_1)

{
  uint uVar1;
  char *_Str;
  undefined1 *puVar2;
  char *unaff_EBX;
  char local_45;
  undefined1 auStack_44 [8];
  uint uStack_3c;
  uint uStack_38;
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167cf50;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  local_45 = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,&local_45);
  uVar1 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar1);
  uStack_4 = 0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,0);
  local_10 = CONCAT31(local_10._1_3_,2);
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffaf);
  uVar1 = std::char_traits<char>::length("");
  uVar1 = FUN_004242c0(&stack0xffffffb0,0,uStack_3c,"",uVar1);
  if (uVar1 == 0) {
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(unaff_EBX);
    }
    uStack_38 = 0xf;
    uStack_3c = 0;
    std::char_traits<char>::assign(&stack0xffffffb4,&stack0xffffffaf);
  }
  else {
    _Str = unaff_EBX;
    if (uStack_38 < 0x10) {
      _Str = &stack0xffffffb4;
    }
    puVar2 = (undefined1 *)atol(_Str);
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(unaff_EBX);
    }
    uStack_38 = 0xf;
    puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
    uStack_3c = 0;
    std::char_traits<char>::assign(&stack0xffffffb4,(char *)&puStack_8);
    puStack_8 = puVar2;
  }
  ExceptionList = pvStack_18;
  return puStack_8;
}


/* [VTable] XMLSection virtual function #21
   VTable: vtable_XMLSection at 017fd22c */

float10 __fastcall XMLSection__vfunc_21(int *param_1)

{
  uint uVar1;
  char ***_String;
  double dVar2;
  undefined1 auStack_50 [3];
  char local_4d;
  char **appcStack_4c [2];
  undefined1 auStack_44 [8];
  uint uStack_3c;
  uint uStack_38;
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167cf50;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  local_4d = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,&local_4d);
  uVar1 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar1);
  uStack_4 = 0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,0);
  local_10 = CONCAT31(local_10._1_3_,2);
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffa7);
  uVar1 = std::char_traits<char>::length("");
  uVar1 = FUN_004242c0(auStack_50,0,uStack_3c,"",uVar1);
  if (uVar1 == 0) {
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(appcStack_4c[0]);
    }
    uStack_38 = 0xf;
    uStack_3c = 0;
    std::char_traits<char>::assign((char *)appcStack_4c,&stack0xffffffa7);
    dVar2 = (double)CONCAT44(uStack_4,puStack_8);
  }
  else {
    _String = (char ***)appcStack_4c[0];
    if (uStack_38 < 0x10) {
      _String = appcStack_4c;
    }
    dVar2 = atof((char *)_String);
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(appcStack_4c[0]);
    }
    uStack_38 = 0xf;
    puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
    uStack_3c = 0;
    std::char_traits<char>::assign((char *)appcStack_4c,(char *)&puStack_8);
  }
  ExceptionList = pvStack_18;
  return (float10)dVar2;
}


/* [VTable] XMLSection virtual function #22
   VTable: vtable_XMLSection at 017fd22c */

void * __thiscall XMLSection__vfunc_22(void *this,void *param_1)

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
  pcVar1 = *(char **)((int)this + 0xc);
  local_11[0] = '\0';
  ExceptionList = &pvStack_c;
  *(undefined4 *)((int)param_1 + 0x18) = 0xf;
  *(undefined4 *)((int)param_1 + 0x14) = 0;
  if (pcVar1 == (char *)0x0) {
    std::char_traits<char>::assign((char *)((int)param_1 + 4),local_11);
    FUN_00437710(param_1,(void *)((int)this + 0x2c),0,0xffffffff);
  }
  else {
    std::char_traits<char>::assign((char *)((int)param_1 + 4),local_11);
    uVar2 = std::char_traits<char>::length(pcVar1);
    FUN_004377d0(param_1,pcVar1,uVar2);
  }
  ExceptionList = pvStack_c;
  return param_1;
}


/* [VTable] XMLSection virtual function #24
   VTable: vtable_XMLSection at 017fd22c
   [String discovery] References: "XMLSection::asVector2" */

undefined4 * __fastcall XMLSection__vfunc_24(int *param_1)

{
  undefined4 uVar1;
  undefined4 *puVar2;
  uint uVar3;
  char ****_Src;
  int iVar4;
  char local_4d;
  char ***apppcStack_4c [2];
  undefined1 auStack_44 [8];
  undefined4 uStack_3c;
  uint uStack_38;
  undefined1 auStack_34 [4];
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined4 *puStack_8;
  undefined4 *puStack_4;
  
  puStack_4 = (undefined4 *)0xffffffff;
  puStack_8 = (undefined4 *)&LAB_0167cf78;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  local_4d = '\0';
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,&local_4d);
  uVar3 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar3);
  puStack_4 = (undefined4 *)0x0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,0);
  local_10._0_1_ = 2;
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffa7);
  _Src = (char ****)apppcStack_4c[0];
  if (uStack_38 < 0x10) {
    _Src = apppcStack_4c;
  }
  iVar4 = sscanf((char *)_Src,"%f%f",&stack0xffffffa8,&stack0xffffffac);
  if (iVar4 == 2) {
    *puStack_8 = 0;
    puStack_8[1] = 0;
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(apppcStack_4c[0]);
    }
  }
  else {
    (**(code **)(*param_1 + 0x2c))(auStack_34);
    local_10._0_1_ = 3;
    _00___FRawStaticIndexBuffer__vfunc_0();
    local_10 = CONCAT31(local_10._1_3_,2);
    if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_30[0]);
    }
    uStack_1c = 0xf;
    uStack_20 = 0;
    std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffa7);
    uVar1 = puStack_4[1];
    *puStack_8 = *puStack_4;
    puStack_8[1] = uVar1;
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(apppcStack_4c[0]);
    }
  }
  puVar2 = puStack_8;
  local_10 = 0xffffffff;
  uStack_38 = 0xf;
  puStack_8 = (undefined4 *)((uint)puStack_8 & 0xffffff00);
  uStack_3c = 0;
  std::char_traits<char>::assign((char *)apppcStack_4c,(char *)&puStack_8);
  ExceptionList = pvStack_18;
  return puVar2;
}


/* [VTable] XMLSection virtual function #25
   VTable: vtable_XMLSection at 017fd22c
   [String discovery] References: "XMLSection::asVector3" */

undefined8 * __fastcall XMLSection__vfunc_25(int *param_1)

{
  undefined4 uVar1;
  undefined8 *puVar2;
  uint uVar3;
  char ****_Src;
  int iVar4;
  undefined4 uStack_54;
  char ***apppcStack_4c [2];
  undefined1 auStack_44 [8];
  undefined4 uStack_3c;
  uint uStack_38;
  undefined1 auStack_34 [4];
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined8 *puStack_8;
  undefined8 *puStack_4;
  
  puStack_4 = (undefined8 *)0xffffffff;
  puStack_8 = (undefined8 *)&LAB_0167cf78;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  uStack_54 = (uint)(uint3)uStack_54;
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,(char *)((int)&uStack_54 + 3));
  uVar3 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar3);
  puStack_4 = (undefined8 *)0x0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,0);
  local_10._0_1_ = 2;
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffa3);
  uStack_54 = 0;
  _Src = (char ****)apppcStack_4c[0];
  if (uStack_38 < 0x10) {
    _Src = apppcStack_4c;
  }
  iVar4 = sscanf((char *)_Src,"%f%f%f",&stack0xffffffa4,&stack0xffffffa8,&uStack_54);
  if (iVar4 == 3) {
    *puStack_8 = 0;
    *(uint *)(puStack_8 + 1) = uStack_54;
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(apppcStack_4c[0]);
    }
  }
  else {
    (**(code **)(*param_1 + 0x2c))(auStack_34);
    local_10._0_1_ = 3;
    _00___FRawStaticIndexBuffer__vfunc_0();
    local_10 = CONCAT31(local_10._1_3_,2);
    if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_30[0]);
    }
    uStack_1c = 0xf;
    uStack_20 = 0;
    std::char_traits<char>::assign((char *)auStack_30,&stack0xffffffa3);
    uVar1 = *(undefined4 *)(puStack_4 + 1);
    *puStack_8 = *puStack_4;
    *(undefined4 *)(puStack_8 + 1) = uVar1;
    local_10 = 0xffffffff;
    if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
      scalable_free(apppcStack_4c[0]);
    }
  }
  puVar2 = puStack_8;
  local_10 = 0xffffffff;
  uStack_38 = 0xf;
  puStack_8 = (undefined8 *)((uint)puStack_8 & 0xffffff00);
  uStack_3c = 0;
  std::char_traits<char>::assign((char *)apppcStack_4c,(char *)&puStack_8);
  ExceptionList = pvStack_18;
  return puVar2;
}


/* [VTable] XMLSection virtual function #26
   VTable: vtable_XMLSection at 017fd22c
   [String discovery] References: "XMLSection::asVector4" */

undefined8 * __fastcall XMLSection__vfunc_26(int *param_1)

{
  undefined8 *puVar1;
  uint uVar2;
  char *****_Src;
  int iVar3;
  undefined8 uVar4;
  undefined4 uStack_58;
  undefined4 auStack_54 [2];
  char ****appppcStack_4c [2];
  undefined1 auStack_44 [8];
  undefined4 uStack_3c;
  uint uStack_38;
  undefined1 auStack_34 [4];
  undefined4 auStack_30 [2];
  undefined1 auStack_28 [4];
  char local_24 [4];
  undefined4 uStack_20;
  uint uStack_1c;
  void *pvStack_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined8 *puStack_8;
  undefined8 *puStack_4;
  
  puStack_4 = (undefined8 *)0xffffffff;
  puStack_8 = (undefined8 *)&LAB_0167cf78;
  pvStack_c = ExceptionList;
  local_10 = 0xf;
  uStack_58 = (uint)(uint3)uStack_58;
  local_14 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_24,(char *)((int)&uStack_58 + 3));
  uVar2 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_28,"",uVar2);
  puStack_4 = (undefined8 *)0x0;
  (**(code **)(*param_1 + 0x58))(auStack_44,auStack_28,0);
  local_10._0_1_ = 2;
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_30[0]);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)auStack_30,&stack0xffffff9f);
  uStack_58 = 0;
  auStack_54[0] = 0;
  _Src = (char *****)appppcStack_4c[0];
  if (uStack_38 < 0x10) {
    _Src = appppcStack_4c;
  }
  iVar3 = sscanf((char *)_Src,"%f%f%f%f",&stack0xffffffa0,&stack0xffffffa4,&uStack_58,auStack_54);
  if (iVar3 == 4) {
    *puStack_8 = 0;
    uVar4 = CONCAT44(auStack_54[0],uStack_58);
  }
  else {
    (**(code **)(*param_1 + 0x2c))(auStack_34);
    local_10._0_1_ = 3;
    _00___FRawStaticIndexBuffer__vfunc_0();
    local_10 = CONCAT31(local_10._1_3_,2);
    if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
      scalable_free(auStack_30[0]);
    }
    uStack_1c = 0xf;
    uStack_20 = 0;
    std::char_traits<char>::assign((char *)auStack_30,&stack0xffffff9f);
    *puStack_8 = *puStack_4;
    uVar4 = puStack_4[1];
  }
  puVar1 = puStack_8;
  puStack_8[1] = uVar4;
  local_10 = 0xffffffff;
  if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
    scalable_free(appppcStack_4c[0]);
  }
  uStack_38 = 0xf;
  puStack_8 = (undefined8 *)((uint)puStack_8 & 0xffffff00);
  uStack_3c = 0;
  std::char_traits<char>::assign((char *)appppcStack_4c,(char *)&puStack_8);
  ExceptionList = pvStack_18;
  return puVar1;
}


/* [VTable] XMLSection virtual function #27
   VTable: vtable_XMLSection at 017fd22c */

undefined8 * __thiscall XMLSection__vfunc_27(void *this,undefined8 *param_1,undefined8 *param_2)

{
  undefined4 uVar1;
  undefined8 *puVar2;
  uint uVar3;
  undefined8 *puVar4;
  undefined8 *puVar5;
  undefined8 uStack_50;
  int iStack_44;
  undefined4 local_40 [4];
  undefined4 local_30;
  uint local_2c;
  int iStack_28;
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  puVar2 = param_1;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167cfb0;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  memset(param_1,0,0x40);
  *(undefined4 *)((int)puVar2 + 0x3c) = DAT_017f7ea0;
  local_2c = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uVar3 = std::char_traits<char>::length("row0");
  FUN_004377d0(&iStack_44,"row0",uVar3);
  puVar5 = param_2;
  uStack_4 = 0;
  puVar4 = PackedSection__unknown_00468b50(this,&uStack_50,&iStack_44,param_2);
  uVar1 = *(undefined4 *)(puVar4 + 1);
  *puVar2 = *puVar4;
  *(undefined4 *)(puVar2 + 1) = uVar1;
  uStack_4 = 0xffffffff;
  if (0xf < local_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_40[0]);
  }
  local_2c = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uStack_10 = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  uVar3 = std::char_traits<char>::length("row1");
  FUN_004377d0(&iStack_28,"row1",uVar3);
  uStack_4 = 1;
  puVar4 = PackedSection__unknown_00468b50(this,&uStack_50,&iStack_28,puVar5 + 2);
  uVar1 = *(undefined4 *)(puVar4 + 1);
  puVar2[2] = *puVar4;
  *(undefined4 *)(puVar2 + 3) = uVar1;
  uStack_4 = 0xffffffff;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  local_2c = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uVar3 = std::char_traits<char>::length("row2");
  FUN_004377d0(&iStack_44,"row2",uVar3);
  uStack_4 = 2;
  puVar4 = PackedSection__unknown_00468b50(this,&uStack_50,&iStack_44,puVar5 + 4);
  uVar1 = *(undefined4 *)(puVar4 + 1);
  puVar2[4] = *puVar4;
  *(undefined4 *)(puVar2 + 5) = uVar1;
  uStack_4 = 0xffffffff;
  if (0xf < local_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_40[0]);
  }
  local_2c = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uStack_10 = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  uVar3 = std::char_traits<char>::length("row3");
  FUN_004377d0(&iStack_28,"row3",uVar3);
  uStack_4 = 3;
  puVar5 = PackedSection__unknown_00468b50(this,&uStack_50,&iStack_28,puVar5 + 6);
  uVar1 = *(undefined4 *)(puVar5 + 1);
  puVar2[6] = *puVar5;
  *(undefined4 *)(puVar2 + 7) = uVar1;
  uStack_4 = 0xffffffff;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_1 = (undefined8 *)((uint)param_1 & 0xffffff00);
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  ExceptionList = pvStack_c;
  return puVar2;
}


/* [VTable] XMLSection virtual function #29
   VTable: vtable_XMLSection at 017fd22c */

void __fastcall XMLSection__vfunc_29(int *param_1)

{
  undefined1 *this;
  uint uVar1;
  char *pcVar2;
  char local_55 [9];
  undefined4 auStack_4c [2];
  undefined1 auStack_44 [4];
  char local_40 [4];
  undefined4 uStack_3c;
  uint uStack_38;
  undefined1 auStack_34 [4];
  undefined4 local_30;
  undefined4 local_2c;
  undefined1 auStack_28 [8];
  uint uStack_20;
  undefined1 uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167cfe9;
  pvStack_c = ExceptionList;
  local_4 = 0;
  local_55[1] = '\0';
  local_55[2] = '\0';
  local_55[3] = '\0';
  local_55[4] = '\0';
  local_2c = 0xf;
  local_55[0] = '\0';
  local_30 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign(local_40,local_55);
  uVar1 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_44,"",uVar1);
  local_4 = 1;
  (**(code **)(*param_1 + 0x58))(auStack_28,auStack_44,0);
  uStack_10 = 3;
  if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_4c[0]);
  }
  uStack_38 = 0xf;
  uStack_3c = 0;
  std::char_traits<char>::assign((char *)auStack_4c,&stack0xffffff9f);
  pcVar2 = (char *)scalable_malloc(uStack_20);
  if (pcVar2 == (char *)0x0) {
    FUN_004099f0((exception *)&stack0xffffffa4);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(&stack0xffffffa4,(ThrowInfo *)&DAT_01d65cc8);
  }
  uVar1 = FUN_00a35440((int)auStack_34,(int)pcVar2,uStack_20);
  this = puStack_8;
  *(undefined4 *)(puStack_8 + 0x18) = 0xf;
  *(undefined4 *)(puStack_8 + 0x14) = 0;
  std::char_traits<char>::assign(puStack_8 + 4,&stack0xffffff9f);
  FUN_004377d0(this,pcVar2,uVar1);
                    /* WARNING: Subroutine does not return */
  scalable_free(pcVar2);
}


/* [VTable] XMLSection virtual function #30
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_30(void *this,char param_1,char *param_2)

{
  undefined1 uVar1;
  char *pcVar2;
  uint uVar3;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  pcVar2 = "true";
  if (param_1 == '\0') {
    pcVar2 = "false";
  }
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,pcVar2);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar3 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar3);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))(&uStack_228);
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #31
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_31(void *this,undefined4 param_1,char *param_2)

{
  undefined1 uVar1;
  uint uVar2;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,param_1);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar2 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar2);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))(&uStack_228);
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #33
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_33(void *this,float param_1,char *param_2)

{
  undefined1 uVar1;
  uint uVar2;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,(double)param_1);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar2 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar2);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))();
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #34
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_34(void *this,undefined8 param_1,char *param_2)

{
  undefined1 uVar1;
  uint uVar2;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,param_1);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar2 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar2);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))();
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #35
   VTable: vtable_XMLSection at 017fd22c */

undefined4 __thiscall XMLSection__vfunc_35(void *this,void *param_1)

{
  void *pvVar1;
  
  *(undefined4 *)((int)this + 0xc) = 0;
  pvVar1 = FUN_00437710((void *)((int)this + 0x2c),param_1,0,0xffffffff);
  return CONCAT31((int3)((uint)pvVar1 >> 8),1);
}


/* [VTable] XMLSection virtual function #37
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_37(void *this,float *param_1,char *param_2)

{
  undefined1 uVar1;
  uint uVar2;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,(double)*param_1,(double)param_1[1]);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar2 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar2);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))();
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #38
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_38(void *this,float *param_1,char *param_2)

{
  undefined1 uVar1;
  uint uVar2;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,(double)*param_1,(double)param_1[1],(double)param_1[2]);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar2 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar2);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))();
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #39
   VTable: vtable_XMLSection at 017fd22c */

undefined1 __thiscall XMLSection__vfunc_39(void *this,float *param_1,char *param_2)

{
  undefined1 uVar1;
  uint uVar2;
  char acStack_22a [2];
  undefined4 uStack_228;
  char acStack_224 [12];
  undefined4 uStack_218;
  uint uStack_214;
  undefined4 uStack_210;
  char local_20c [508];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167d00b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  sprintf(local_20c,param_2,(double)*param_1,(double)param_1[1],(double)param_1[2],
          (double)param_1[3]);
  uStack_210 = 0xf;
  acStack_22a[0] = '\0';
  uStack_214 = 0;
  std::char_traits<char>::assign(acStack_224,acStack_22a);
  uVar2 = std::char_traits<char>::length(local_20c);
  FUN_004377d0(&uStack_228,local_20c,uVar2);
  uStack_4 = 0;
  uVar1 = (**(code **)(*(int *)this + 0x8c))();
  puStack_8 = (undefined1 *)0xffffffff;
  if (0xf < uStack_214) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_228);
  }
  uStack_214 = 0xf;
  uStack_218 = 0;
  std::char_traits<char>::assign((char *)&uStack_228,&stack0xfffffdd2);
  ExceptionList = pvStack_10;
  return uVar1;
}


/* [VTable] XMLSection virtual function #40
   VTable: vtable_XMLSection at 017fd22c */

uint __thiscall XMLSection__vfunc_40(void *this,uint param_1,void *param_2)

{
  void *pvVar1;
  undefined1 uVar2;
  byte bVar3;
  uint uVar4;
  uint uVar5;
  undefined4 extraout_EAX;
  char local_45;
  int iStack_44;
  undefined4 local_40 [4];
  undefined4 local_30;
  uint local_2c;
  int iStack_28;
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0167cfb0;
  pvStack_c = ExceptionList;
  local_2c = 0xf;
  local_45 = '\0';
  local_30 = 0;
  ExceptionList = &pvStack_c;
  std::char_traits<char>::assign((char *)local_40,&local_45);
  uVar4 = std::char_traits<char>::length("row0");
  FUN_004377d0(&iStack_44,"row0",uVar4);
  pvVar1 = param_2;
  uVar4 = param_1;
  uStack_4 = 0;
  uVar2 = XMLSection__unknown_00468fa0(this,&iStack_44,param_1,param_2);
  param_2 = (void *)(CONCAT31(param_2._1_3_,uVar2) & 0xffffff01);
  uStack_4 = 0xffffffff;
  if (0xf < local_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_40[0]);
  }
  local_2c = 0xf;
  param_1 = param_1 & 0xffffff00;
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uStack_10 = 0xf;
  param_1 = param_1 & 0xffffff00;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  uVar5 = std::char_traits<char>::length("row1");
  FUN_004377d0(&iStack_28,"row1",uVar5);
  uStack_4 = 1;
  bVar3 = XMLSection__unknown_00468fa0(this,&iStack_28,uVar4 + 0x10,pvVar1);
  param_2 = (void *)CONCAT31(param_2._1_3_,(byte)param_2 & bVar3);
  uStack_4 = 0xffffffff;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_1 = param_1 & 0xffffff00;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  local_2c = 0xf;
  param_1 = param_1 & 0xffffff00;
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uVar5 = std::char_traits<char>::length("row2");
  FUN_004377d0(&iStack_44,"row2",uVar5);
  uStack_4 = 2;
  bVar3 = XMLSection__unknown_00468fa0(this,&iStack_44,uVar4 + 0x20,pvVar1);
  param_2 = (void *)CONCAT31(param_2._1_3_,(byte)param_2 & bVar3);
  uStack_4 = 0xffffffff;
  if (0xf < local_2c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_40[0]);
  }
  local_2c = 0xf;
  param_1 = param_1 & 0xffffff00;
  local_30 = 0;
  std::char_traits<char>::assign((char *)local_40,(char *)&param_1);
  uStack_10 = 0xf;
  param_1 = param_1 & 0xffffff00;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  uVar5 = std::char_traits<char>::length("row3");
  FUN_004377d0(&iStack_28,"row3",uVar5);
  uStack_4 = 3;
  bVar3 = XMLSection__unknown_00468fa0(this,&iStack_28,uVar4 + 0x30,pvVar1);
  param_2 = (void *)CONCAT31(param_2._1_3_,(byte)param_2 & bVar3);
  uStack_4 = 0xffffffff;
  if (0xf < uStack_10) {
                    /* WARNING: Subroutine does not return */
    scalable_free(auStack_24[0]);
  }
  uStack_10 = 0xf;
  param_1 = param_1 & 0xffffff00;
  uStack_14 = 0;
  std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
  ExceptionList = pvStack_c;
  return CONCAT31((int3)((uint)extraout_EAX >> 8),(byte)param_2);
}


/* [VTable] XMLSection virtual function #28
   VTable: vtable_XMLSection at 017fd22c */

undefined4 * __thiscall XMLSection__vfunc_28(void *this,undefined4 *param_1)

{
  undefined4 uVar1;
  void *this_00;
  undefined4 *puVar2;
  uint unaff_EBX;
  basic_ostream<char,struct_std::char_traits<char>_> *unaff_ESI;
  undefined4 *unaff_retaddr;
  int *piVar3;
  int in_stack_ffffff40;
  undefined4 in_stack_ffffff44;
  undefined4 in_stack_ffffff48;
  char in_stack_ffffff4c;
  basic_istream<char,struct_std::char_traits<char>_> abStack_b0 [4];
  undefined1 local_ac [8];
  basic_ostream<char,struct_std::char_traits<char>_> local_a4 [72];
  basic_ios<char,struct_std::char_traits<char>_> abStack_5c [76];
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0167d156;
  pvStack_c = ExceptionList;
  piVar3 = (int *)0x0;
  ExceptionList = &pvStack_c;
  FUN_0046b6b0(local_ac,3,1);
  local_4 = 1;
  uVar1 = FUN_0046e4d0(this,(int *)local_a4,0,unaff_ESI,unaff_EBX,piVar3,in_stack_ffffff40,
                       in_stack_ffffff44,in_stack_ffffff48,in_stack_ffffff4c);
  if ((char)uVar1 == '\0') {
    *param_1 = 0;
    local_4 = CONCAT31(local_4._1_3_,1);
  }
  else {
    this_00 = (void *)scalable_malloc(0x14);
    if (this_00 == (void *)0x0) {
      FUN_004099f0((exception *)&stack0xffffff48);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(&stack0xffffff48,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_4 = CONCAT31(local_4._1_3_,3);
    piVar3 = (int *)std::basic_ostream<char,struct_std::char_traits<char>_>::tellp(local_a4);
    puVar2 = FUN_00472380(this_00,abStack_b0,piVar3[2] + *piVar3);
    puStack_8._0_1_ = 1;
    if (puVar2 != (undefined4 *)0x0) {
      FUN_00457e40((int)puVar2);
    }
    puStack_8._0_1_ = 5;
    FUN_00458600(unaff_retaddr,(int *)&stack0xffffff3c);
    puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,1);
    FUN_004585a0((int *)&stack0xffffff3c);
    param_1 = unaff_retaddr;
  }
  puStack_8 = (undefined1 *)((uint)puStack_8 & 0xffffff00);
  FUN_0046b5f0((int)abStack_5c);
  std::basic_ios<char,struct_std::char_traits<char>_>::
  ~basic_ios<char,struct_std::char_traits<char>_>(abStack_5c);
  ExceptionList = pvStack_10;
  return param_1;
}


/* [VTable] XMLSection virtual function #23
   VTable: vtable_XMLSection at 017fd22c */

undefined1 * __fastcall XMLSection__vfunc_23(int *param_1)

{
  undefined1 *puVar1;
  uint uVar2;
  int iVar3;
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
  uVar2 = std::char_traits<char>::length("");
  FUN_004377d0(auStack_44,"",uVar2);
  uStack_4 = 1;
  iVar3 = (**(code **)(*param_1 + 0x58))(auStack_28,auStack_44,0);
  puVar1 = puStack_8;
  uStack_10 = 2;
  FUN_0046ed40(puStack_8,iVar3);
  uStack_10 = 1;
  if (0xf < uStack_1c) {
                    /* WARNING: Subroutine does not return */
    scalable_free(local_30);
  }
  uStack_1c = 0xf;
  uStack_20 = 0;
  std::char_traits<char>::assign((char *)&local_30,&stack0xffffffab);
  uStack_10 = 0;
  if (0xf < uStack_38) {
                    /* WARNING: Subroutine does not return */
    scalable_free(uStack_4c);
  }
  uStack_38 = 0xf;
  uStack_3c = 0;
  std::char_traits<char>::assign((char *)&uStack_4c,&stack0xffffffab);
  ExceptionList = pvStack_18;
  return puVar1;
}


/* [VTable] XMLSection virtual function #20
   VTable: vtable_XMLSection at 017fd22c */

void __thiscall XMLSection__vfunc_20(void *this,float param_1)

{
  (**(code **)(*(int *)this + 0x54))((double)param_1);
  return;
}


/* WARNING: Removing unreachable block (ram,0x0046f8a2) */
/* WARNING: Removing unreachable block (ram,0x0046f8ed) */
/* [VTable] XMLSection virtual function #9
   VTable: vtable_XMLSection at 017fd22c */

void __thiscall XMLSection__vfunc_9(void *this,uint param_1)

{
  int *piVar1;
  uint uVar2;
  uint uVar3;
  int *piVar4;
  int iVar5;
  char *pcVar6;
  uint uVar7;
  uint uVar8;
  char *pcVar9;
  bool bVar10;
  undefined4 uStack_30;
  int *local_2c;
  undefined1 local_28 [4];
  undefined4 auStack_24 [4];
  undefined4 uStack_14;
  uint uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d5d28;
  local_c = ExceptionList;
  local_2c = *(int **)((int)this + 0x4c);
  ExceptionList = &local_c;
  uVar3 = param_1;
  if (*(int **)((int)this + 0x50) < local_2c) {
    ExceptionList = &local_c;
    _invalid_parameter_noinfo();
    uVar3 = param_1;
  }
  while( true ) {
    piVar4 = local_2c;
    piVar1 = *(int **)((int)this + 0x50);
    if (piVar1 < *(int **)((int)this + 0x4c)) {
      _invalid_parameter_noinfo();
    }
    if (piVar4 == piVar1) {
      ExceptionList = local_c;
      return;
    }
    if (*(int **)((int)this + 0x50) <= piVar4) {
      _invalid_parameter_noinfo();
    }
    iVar5 = (**(code **)(*(int *)*piVar4 + 0x2c))(local_28);
    uStack_4 = 0;
    uVar7 = *(uint *)(uVar3 + 0x14);
    if (*(uint *)(uVar3 + 0x18) < 0x10) {
      pcVar9 = (char *)(uVar3 + 4);
    }
    else {
      pcVar9 = *(char **)(uVar3 + 4);
    }
    uVar2 = *(uint *)(iVar5 + 0x14);
    uVar8 = uVar2;
    if (uVar7 <= uVar2) {
      uVar8 = uVar7;
    }
    if (*(uint *)(iVar5 + 0x18) < 0x10) {
      pcVar6 = (char *)(iVar5 + 4);
    }
    else {
      pcVar6 = *(char **)(iVar5 + 4);
    }
    iVar5 = std::char_traits<char>::compare(pcVar6,pcVar9,uVar8);
    bVar10 = false;
    if (iVar5 == 0) {
      if (uVar2 < uVar7) {
        uVar7 = 0xffffffff;
      }
      else {
        uVar7 = (uint)(uVar2 != uVar7);
      }
      bVar10 = uVar7 == 0;
    }
    uStack_4 = 0xffffffff;
    if (0xf < uStack_10) break;
    uStack_10 = 0xf;
    param_1 = param_1 & 0xffffff00;
    uStack_14 = 0;
    std::char_traits<char>::assign((char *)auStack_24,(char *)&param_1);
    if (bVar10) {
      FUN_0046f1f0((void *)((int)this + 0x48),&uStack_30,(void *)((int)this + 0x48),local_2c);
      ExceptionList = local_c;
      return;
    }
    if (*(int **)((int)this + 0x50) <= local_2c) {
      _invalid_parameter_noinfo();
    }
    local_2c = local_2c + 1;
  }
                    /* WARNING: Subroutine does not return */
  scalable_free(auStack_24[0]);
}


/* WARNING: Removing unreachable block (ram,0x0046fa05) */
/* [VTable] XMLSection virtual function #8
   VTable: vtable_XMLSection at 017fd22c */

void __thiscall XMLSection__vfunc_8(void *this,int param_1)

{
  int *piVar1;
  int *piVar2;
  undefined4 local_14 [2];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0167bca8;
  local_c = ExceptionList;
  local_4 = 0;
  piVar2 = *(int **)((int)this + 0x4c);
  ExceptionList = &local_c;
  if (*(int **)((int)this + 0x50) < piVar2) {
    ExceptionList = &local_c;
    _invalid_parameter_noinfo();
  }
  while( true ) {
    piVar1 = *(int **)((int)this + 0x50);
    if (piVar1 < *(int **)((int)this + 0x4c)) {
      _invalid_parameter_noinfo();
    }
    if (piVar2 == piVar1) goto LAB_0046fa34;
    if (*(int **)((int)this + 0x50) <= piVar2) {
      _invalid_parameter_noinfo();
    }
    if (*piVar2 == param_1) break;
    if (*(int **)((int)this + 0x50) <= piVar2) {
      _invalid_parameter_noinfo();
    }
    piVar2 = piVar2 + 1;
  }
  FUN_0046f1f0((void *)((int)this + 0x48),local_14,(void *)((int)this + 0x48),piVar2);
LAB_0046fa34:
  local_4 = 0xffffffff;
  FUN_004585a0(&param_1);
  ExceptionList = local_c;
  return;
}


/* Also in vtable: XMLSection (slot 0) */

undefined4 * __thiscall XMLSection__vfunc_0(void *this,byte param_1)

{
  FUN_0046fa60(this);
  if ((param_1 & 1) != 0) {
    FUN_0046f5f0(this);
  }
  return this;
}


/* [VTable] XMLSection virtual function #10
   VTable: vtable_XMLSection at 017fd22c */

void __fastcall XMLSection__vfunc_10(int param_1)

{
  void *this;
  int *piVar1;
  int *piVar2;
  int local_8 [2];
  
  this = (void *)(param_1 + 0x48);
  piVar1 = *(int **)(param_1 + 0x50);
  if (piVar1 < *(int **)(param_1 + 0x4c)) {
    _invalid_parameter_noinfo();
  }
  piVar2 = *(int **)(param_1 + 0x4c);
  if (*(int **)(param_1 + 0x50) < piVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0046f260(this,local_8,(int)this,piVar2,(int)this,piVar1);
  return;
}


/* [VTable] XMLSection virtual function #5
   VTable: vtable_XMLSection at 017fd22c */

undefined4 * __thiscall XMLSection__vfunc_5(void *this,undefined4 *param_1,void *param_2)

{
  undefined4 *puVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017949f1;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = FUN_0046fc40(this,(undefined4 *)0x0,-1);
  FUN_00437710(puVar1 + 4,param_2,0,0xffffffff);
  *param_1 = puVar1;
  if (puVar1 != (undefined4 *)0x0) {
    FUN_00457e40((int)puVar1);
  }
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] XMLSection virtual function #6
   VTable: vtable_XMLSection at 017fd22c */

undefined4 * __thiscall
XMLSection__vfunc_6(void *this,undefined4 *param_1,void *param_2,int param_3)

{
  undefined4 *puVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017949f1;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  puVar1 = FUN_0046fc40(this,(undefined4 *)0x0,param_3);
  FUN_00437710(puVar1 + 4,param_2,0,0xffffffff);
  *param_1 = puVar1;
  if (puVar1 != (undefined4 *)0x0) {
    FUN_00457e40((int)puVar1);
  }
  ExceptionList = local_c;
  return param_1;
}




/* === From standalone functions (72 functions) === */

void _A0xea36dab8_Win32AssertHandler__vfunc_0(void)

{
  int in_stack_00000010;
  basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> local_5c [28];
  undefined1 auStack_40 [44];
  void *local_14;
  undefined1 *puStack_10;
  undefined4 local_c;
  
  puStack_10 = &LAB_01799de1;
  local_14 = ExceptionList;
  ExceptionList = &local_14;
  local_c = 0;
  if (in_stack_00000010 == 0) {
    FUN_004028e0();
    FUN_004027a0(0);
  }
  else {
    FUN_004028e0();
    FUN_004027a0(0);
  }
  std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::
  basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>
            (local_5c,"Assertion failed");
  local_c = CONCAT31(local_c._1_3_,1);
  FUN_00402530(auStack_40,local_5c);
                    /* WARNING: Subroutine does not return */
  _CxxThrowException(auStack_40,(ThrowInfo *)&DAT_01d65974);
}


/* Also in vtable: wxEventLoopBase_wxUnrealCallbacks (slot 0) */

undefined4 * __thiscall wxEventLoopBase_wxUnrealCallbacks__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = wxEventLoopBase::wxUnrealCallbacks::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: tbb_task (slot 0) */

undefined4 * __thiscall tbb_task__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = tbb::task::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] detail__RootTask virtual function #1
   VTable: vtable_detail__RootTask at 0181b900 */

undefined4 __fastcall detail__RootTask__vfunc_1(int param_1)

{
  undefined4 uVar1;
  void *local_14;
  undefined *puStack_10;
  undefined *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &DAT_01c25530;
  puStack_10 = &DAT_01237520;
  local_14 = ExceptionList;
  if (*(char *)(param_1 + 0x10) == '\0') {
    ExceptionList = &local_14;
    *(undefined4 *)(param_1 + -0x10) = 1;
    uVar1 = (**(code **)(param_1 + 4))(*(undefined4 *)(param_1 + 8));
    **(undefined4 **)(param_1 + 0xc) = uVar1;
    ExceptionList = local_14;
    return 0;
  }
  local_8 = 0;
  ExceptionList = &local_14;
  FUN_00507ef0(param_1);
  ExceptionList = local_14;
  return 0;
}


/* Also in vtable: NVSystemOptionPolicyBool___SystemOption (slot 0) */

undefined4 * __thiscall _NVSystemOptionPolicyBool___SystemOption__vfunc_0(void *this,byte param_1)

{
  FUN_00577780(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] 00_07__00_U__TStaticMeshFullVertex___TD3DResourceArray virtual function #1
   VTable: vtable__00_07__00_U__TStaticMeshFullVertex___TD3DResourceArray at 0184e604
   Also in vtable: 00_07_UFSoftSkinVertex___TD3DResourceArray (slot 1)
   Also in vtable: 07__0A_UFModelVertex___TD3DResourceArray (slot 1) */

int __fastcall _0_07__00_U__TStaticMeshFullVertex___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) * 0x28;
}


/* [VTable] 00_07__01_U__TStaticMeshFullVertex___TD3DResourceArray virtual function #1
   VTable: vtable__00_07__01_U__TStaticMeshFullVertex___TD3DResourceArray at 0184e618 */

int __fastcall _0_07__01_U__TStaticMeshFullVertex___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) * 0x30;
}


/* [VTable] 00_07__02_U__TStaticMeshFullVertex___TD3DResourceArray virtual function #1
   VTable: vtable__00_07__02_U__TStaticMeshFullVertex___TD3DResourceArray at 0184e62c */

int __fastcall _0_07__02_U__TStaticMeshFullVertex___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) * 0x38;
}


/* [VTable] 00_07__03_U__TStaticMeshFullVertex___TD3DResourceArray virtual function #1
   VTable: vtable__00_07__03_U__TStaticMeshFullVertex___TD3DResourceArray at 0184e640 */

int __fastcall _0_07__03_U__TStaticMeshFullVertex___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) << 6;
}


/* [VTable] 00_07_UFShadowVertex___TD3DResourceArray virtual function #1
   VTable: vtable__00_07_UFShadowVertex___TD3DResourceArray at 0184e674 */

int __fastcall _0_07_UFShadowVertex___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) << 4;
}


/* [VTable] 00_07_UFPackedNormal___TD3DResourceArray virtual function #1
   VTable: vtable__00_07_UFPackedNormal___TD3DResourceArray at 0184e688
   Also in vtable: 07_M_0A___TD3DResourceArray (slot 1) */

int __fastcall _0_07_UFPackedNormal___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) * 4;
}


/* [VTable] 07__0A_UFVector2D___TD3DResourceArray virtual function #1
   VTable: vtable__07__0A_UFVector2D___TD3DResourceArray at 0184e69c */

int __fastcall _7__0A_UFVector2D___TD3DResourceArray__vfunc_1(int param_1)

{
  return *(int *)(param_1 + 8) * 8;
}


/* [VTable] 00___TStaticMeshVertexData virtual function #1
   VTable: vtable__00___TStaticMeshVertexData at 0184e77c */

void __thiscall _0___TStaticMeshVertexData__vfunc_1(void *this,uint param_1)

{
  uint uVar1;
  
  uVar1 = *(uint *)((int)this + 0xc);
  if (uVar1 < param_1) {
    FUN_0041a950((void *)((int)this + 8),param_1 - uVar1,0x28,8);
    return;
  }
  if (param_1 < uVar1) {
    FUN_00601710((void *)((int)this + 8),param_1,uVar1 - param_1);
  }
  return;
}


/* [VTable] 01___TStaticMeshVertexData virtual function #1
   VTable: vtable__01___TStaticMeshVertexData at 0184e7ac */

void __thiscall _1___TStaticMeshVertexData__vfunc_1(void *this,uint param_1)

{
  uint uVar1;
  
  uVar1 = *(uint *)((int)this + 0xc);
  if (uVar1 < param_1) {
    FUN_0041a950((void *)((int)this + 8),param_1 - uVar1,0x30,8);
    return;
  }
  if (param_1 < uVar1) {
    FUN_00601790((void *)((int)this + 8),param_1,uVar1 - param_1);
  }
  return;
}


/* [VTable] 02___TStaticMeshVertexData virtual function #2
   VTable: vtable__02___TStaticMeshVertexData at 0184e7dc */

undefined4 _2___TStaticMeshVertexData__vfunc_2(void)

{
  return 0x38;
}


/* [VTable] 02___TStaticMeshVertexData virtual function #1
   VTable: vtable__02___TStaticMeshVertexData at 0184e7dc */

void __thiscall _2___TStaticMeshVertexData__vfunc_1(void *this,uint param_1)

{
  uint uVar1;
  
  uVar1 = *(uint *)((int)this + 0xc);
  if (uVar1 < param_1) {
    FUN_0041a950((void *)((int)this + 8),param_1 - uVar1,0x38,8);
    return;
  }
  if (param_1 < uVar1) {
    FUN_008f0240((void *)((int)this + 8),param_1,uVar1 - param_1);
  }
  return;
}


/* [VTable] 03___TStaticMeshVertexData virtual function #2
   VTable: vtable__03___TStaticMeshVertexData at 0184e80c */

undefined4 _3___TStaticMeshVertexData__vfunc_2(void)

{
  return 0x40;
}


/* [VTable] 03___TStaticMeshVertexData virtual function #1
   VTable: vtable__03___TStaticMeshVertexData at 0184e80c */

void __thiscall _3___TStaticMeshVertexData__vfunc_1(void *this,uint param_1)

{
  uint uVar1;
  
  uVar1 = *(uint *)((int)this + 0xc);
  if (uVar1 < param_1) {
    FUN_0041a950((void *)((int)this + 8),param_1 - uVar1,0x40,8);
    return;
  }
  if (param_1 < uVar1) {
    FUN_005fb180((void *)((int)this + 8),param_1,uVar1 - param_1);
  }
  return;
}


/* Also in vtable: 00___TStaticMeshVertexData (slot 0) */

undefined4 * __thiscall _00___TStaticMeshVertexData__vfunc_0(void *this,byte param_1)

{
  FUN_00604890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: 01___TStaticMeshVertexData (slot 0) */

undefined4 * __thiscall _01___TStaticMeshVertexData__vfunc_0(void *this,byte param_1)

{
  FUN_00604bb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: 02___TStaticMeshVertexData (slot 0) */

undefined4 * __thiscall _02___TStaticMeshVertexData__vfunc_0(void *this,byte param_1)

{
  FUN_00605000(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: 03___TStaticMeshVertexData (slot 0) */

undefined4 * __thiscall _03___TStaticMeshVertexData__vfunc_0(void *this,byte param_1)

{
  FUN_00605330(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* [VTable] 00___FRawStaticIndexBuffer virtual function #2
   VTable: vtable__00___FRawStaticIndexBuffer at 0184e87c */

void __fastcall _0___FRawStaticIndexBuffer__vfunc_2(int *param_1)

{
  int *piVar1;
  int *piVar2;
  int *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016952e8;
  local_c = ExceptionList;
  if (param_1[8] != 0) {
    ExceptionList = &local_c;
    local_10 = param_1;
    piVar2 = FUN_00ecc4f0((int *)&local_10,2,(LPCSTR)(param_1[8] * 2),param_1 + 6,0);
    local_4 = 0;
    piVar2 = (int *)*piVar2;
    piVar1 = (int *)param_1[5];
    param_1[5] = (int)piVar2;
    if (piVar2 != (int *)0x0) {
      (**(code **)(*piVar2 + 4))(piVar2);
    }
    if (piVar1 != (int *)0x0) {
      (**(code **)(*piVar1 + 8))(piVar1);
    }
    local_4 = 0xffffffff;
    if (local_10 != (int *)0x0) {
      (**(code **)(*local_10 + 8))(local_10);
    }
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] 00___TStaticMeshVertexData virtual function #5
   VTable: vtable__00___TStaticMeshVertexData at 0184e77c */

void __thiscall _0___TStaticMeshVertexData__vfunc_5(void *this,int *param_1)

{
  FUN_00605bb0((void *)((int)this + 8),param_1);
  return;
}


/* [VTable] 01___TStaticMeshVertexData virtual function #5
   VTable: vtable__01___TStaticMeshVertexData at 0184e7ac */

void __thiscall _1___TStaticMeshVertexData__vfunc_5(void *this,int *param_1)

{
  FUN_00605c60((void *)((int)this + 8),param_1);
  return;
}


/* [VTable] 02___TStaticMeshVertexData virtual function #5
   VTable: vtable__02___TStaticMeshVertexData at 0184e7dc */

void __thiscall _2___TStaticMeshVertexData__vfunc_5(void *this,int *param_1)

{
  FUN_00605d00((void *)((int)this + 8),param_1);
  return;
}


/* [VTable] 03___TStaticMeshVertexData virtual function #5
   VTable: vtable__03___TStaticMeshVertexData at 0184e80c */

void __thiscall _3___TStaticMeshVertexData__vfunc_5(void *this,int *param_1)

{
  FUN_00605dc0((void *)((int)this + 8),param_1);
  return;
}


/* Also in vtable: A0x45c6b2f3_G___FDecalIndexBuffer (slot 0) */

void __thiscall _A0x45c6b2f3_G___FDecalIndexBuffer__vfunc_0(void *this,void *param_1)

{
  memcpy(param_1,*(void **)((int)this + 4),*(int *)((int)this + 8) * 2);
  return;
}


/* [VTable] 07__0A_UFVector2D___TD3DResourceArray virtual function #2
   VTable: vtable__07__0A_UFVector2D___TD3DResourceArray at 0184e69c
   Also in vtable: 07_M_0A___TD3DResourceArray (slot 2)
   Also in vtable: 07__0A_UFModelVertex___TD3DResourceArray (slot 2) */

void __fastcall _7__0A_UFVector2D___TD3DResourceArray__vfunc_2(int param_1)

{
  int iVar1;
  undefined4 uVar2;
  
  if (((DAT_01ead7ac == 0) && (DAT_01ead7b0 == 0)) &&
     (*(undefined4 *)(param_1 + 8) = 0, *(int *)(param_1 + 0xc) != 0)) {
    iVar1 = *(int *)(param_1 + 4);
    *(undefined4 *)(param_1 + 0xc) = 0;
    if (iVar1 != 0) {
      if (DAT_01ea5778 == (int *)0x0) {
        FUN_00416660();
      }
      uVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar1,0,8);
      *(undefined4 *)(param_1 + 4) = uVar2;
    }
  }
  return;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #1
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

void __thiscall cme_hwprofile_cmebase__NVP__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #2
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

void cme_hwprofile_cmebase__NVP__vfunc_2(void)

{
  return;
}


/* [VTable] cme_hwprofile_cmebase__ErrorBase virtual function #2
   VTable: vtable_cme_hwprofile_cmebase__ErrorBase at 01929e60 */

void cme_hwprofile_cmebase__ErrorBase__vfunc_2(void)

{
  return;
}


/* [VTable] cme_hwprofile_xsd__hexBinary virtual function #1
   VTable: vtable_cme_hwprofile_xsd__hexBinary at 01929e3c */

void __fastcall cme_hwprofile_xsd__hexBinary__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 8) = 0;
  *(undefined4 *)(param_1 + 4) = 0;
  return;
}


/* [VTable] cme_hwprofile_xsd__hexBinary virtual function #2
   VTable: vtable_cme_hwprofile_xsd__hexBinary at 01929e3c */

void __thiscall cme_hwprofile_xsd__hexBinary__vfunc_2(void *this,int param_1)

{
  if (*(int *)((int)this + 4) != 0) {
    FUN_00a3b0c0(param_1,(uint)this,(uint *)((int)this + 4),1,9);
  }
  return;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #4
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

void __thiscall
cme_hwprofile_cmebase__NVP__vfunc_4(void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  FUN_00a6f210(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] cme_hwprofile_cmebase__ErrorBase virtual function #1
   VTable: vtable_cme_hwprofile_cmebase__ErrorBase at 01929e60 */

void __thiscall cme_hwprofile_cmebase__ErrorBase__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x24) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::erase
            ((basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> *)
             ((int)this + 8),0,*(uint *)npos_exref);
  return;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #3
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

undefined4 __thiscall
cme_hwprofile_cmebase__NVP__vfunc_3(void *this,uint param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  undefined4 uVar2;
  
  iVar1 = FUN_00a40f40(param_1,(uint)this,(uint *)0x0,0,param_2,0x11);
  iVar1 = (**(code **)(*(int *)this + 0x10))(param_1,param_2,iVar1,param_3);
  if (iVar1 != 0) {
    return *(undefined4 *)(param_1 + 0x16138);
  }
  uVar2 = FUN_00a70430(param_1);
  return uVar2;
}


/* [VTable] cme_hwprofile_xsd__hexBinary virtual function #3
   VTable: vtable_cme_hwprofile_xsd__hexBinary at 01929e3c */

undefined4 __thiscall
cme_hwprofile_xsd__hexBinary__vfunc_3(void *this,uint param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  undefined4 uVar2;
  
  iVar1 = FUN_00a40f40(param_1,(uint)this,(uint *)((int)this + 4),1,param_2,9);
  iVar1 = (**(code **)(*(int *)this + 0x10))(param_1,param_2,iVar1,param_3);
  if (iVar1 != 0) {
    return *(undefined4 *)(param_1 + 0x16138);
  }
  uVar2 = FUN_00a70430(param_1);
  return uVar2;
}


/* [VTable] cme_hwprofile_cmehwprofile__Config virtual function #2
   VTable: vtable_cme_hwprofile_cmehwprofile__Config at 01929ecc */

void __thiscall cme_hwprofile_cmehwprofile__Config__vfunc_2(void *this,int param_1)

{
  FUN_00a711d0(param_1,(int)this + 4);
  return;
}


/* [VTable] cme_hwprofile_cmehwprofile__Device virtual function #2
   VTable: vtable_cme_hwprofile_cmehwprofile__Device at 01929ea8 */

void __thiscall cme_hwprofile_cmehwprofile__Device__vfunc_2(void *this,int param_1)

{
  FUN_00a712f0(param_1,(int)this + 4);
  return;
}


/* [VTable] cme_hwprofile_cmehwprofile__Config virtual function #1
   VTable: vtable_cme_hwprofile_cmehwprofile__Config at 01929ecc */

void __thiscall cme_hwprofile_cmehwprofile__Config__vfunc_1(void *this,undefined4 param_1)

{
  void *this_00;
  void *pvVar1;
  void *pvVar2;
  int local_8 [2];
  
  this_00 = (void *)((int)this + 4);
  *(undefined4 *)((int)this + 0x14) = param_1;
  pvVar1 = *(void **)((int)this + 0xc);
  if (pvVar1 < *(void **)((int)this + 8)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 8);
  if (*(void **)((int)this + 0xc) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(this_00,local_8,(int)this_00,pvVar2,(int)this_00,pvVar1);
  return;
}


/* [VTable] cme_hwprofile_cmehwprofile__Device virtual function #1
   VTable: vtable_cme_hwprofile_cmehwprofile__Device at 01929ea8 */

void __thiscall cme_hwprofile_cmehwprofile__Device__vfunc_1(void *this,undefined4 param_1)

{
  void *this_00;
  void *pvVar1;
  void *pvVar2;
  int local_8 [2];
  
  this_00 = (void *)((int)this + 4);
  *(undefined4 *)((int)this + 0x30) = param_1;
  pvVar1 = *(void **)((int)this + 0xc);
  if (pvVar1 < *(void **)((int)this + 8)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 8);
  if (*(void **)((int)this + 0xc) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(this_00,local_8,(int)this_00,pvVar2,(int)this_00,pvVar1);
  std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::erase
            ((basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> *)
             ((int)this + 0x14),0,*(uint *)npos_exref);
  return;
}


/* [VTable] cme_hwprofile_cmebase__ErrorBase virtual function #6
   VTable: vtable_cme_hwprofile_cmebase__ErrorBase at 01929e60 */

void __thiscall
cme_hwprofile_cmebase__ErrorBase__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  FUN_00a74220(param_1,param_2,this,param_3);
  return;
}


/* [VTable] cme_hwprofile_xsd__hexBinary virtual function #6
   VTable: vtable_cme_hwprofile_xsd__hexBinary at 01929e3c */

void __thiscall
cme_hwprofile_xsd__hexBinary__vfunc_6(void *this,char *param_1,byte *param_2,byte *param_3)

{
  FUN_00a744e0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #6
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

void __thiscall
cme_hwprofile_cmebase__NVP__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  FUN_00a76020(param_1,param_2,this,param_3);
  return;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #5
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

int * __thiscall
cme_hwprofile_cmebase__NVP__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = FUN_00a76020(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    FUN_00a77ca0(param_1);
  }
  return piVar1;
}


/* [VTable] cme_hwprofile_cmebase__ErrorBase virtual function #5
   VTable: vtable_cme_hwprofile_cmebase__ErrorBase at 01929e60 */

int * __thiscall
cme_hwprofile_cmebase__ErrorBase__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = FUN_00a74220(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    FUN_00a77ca0(param_1);
  }
  return piVar1;
}


/* [VTable] cme_hwprofile_xsd__hexBinary virtual function #5
   VTable: vtable_cme_hwprofile_xsd__hexBinary at 01929e3c */

int * __thiscall
cme_hwprofile_xsd__hexBinary__vfunc_5(void *this,char *param_1,byte *param_2,byte *param_3)

{
  int *piVar1;
  
  piVar1 = FUN_00a744e0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    FUN_00a77ca0(param_1);
  }
  return piVar1;
}


/* [VTable] cme_hwprofile_cmebase__NVP virtual function #7
   VTable: vtable_cme_hwprofile_cmebase__NVP at 01929e84 */

undefined4 * __thiscall cme_hwprofile_cmebase__NVP__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = cme_hwprofile::cmebase__NVP::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00a6e360);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] cme_hwprofile_cmebase__ErrorBase virtual function #7
   VTable: vtable_cme_hwprofile_cmebase__ErrorBase at 01929e60 */

undefined4 * __thiscall cme_hwprofile_cmebase__ErrorBase__vfunc_7(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d817c;
  local_c = ExceptionList;
  if ((param_1 & 2) == 0) {
    ExceptionList = &local_c;
    *(undefined ***)this = cme_hwprofile::cmebase__ErrorBase::vftable;
    local_4 = 0xffffffff;
    std::basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>::
    ~basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_>
              ((basic_string<char,struct_std::char_traits<char>,class_std::allocator<char>_> *)
               ((int)this + 8));
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    ExceptionList = local_c;
    return this;
  }
  ExceptionList = &local_c;
  _eh_vector_destructor_iterator_(this,0x28,*(int *)((int)this + -4),FUN_00a6e300);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  ExceptionList = local_c;
  return (undefined4 *)((int)this + -4);
}


/* [VTable] cme_hwprofile_xsd__hexBinary virtual function #7
   VTable: vtable_cme_hwprofile_xsd__hexBinary at 01929e3c */

undefined4 * __thiscall cme_hwprofile_xsd__hexBinary__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = cme_hwprofile::xsd__hexBinary::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0xc,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00a6e290);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] cme_hwprofile_cmehwprofile__Config virtual function #7
   VTable: vtable_cme_hwprofile_cmehwprofile__Config at 01929ecc */

undefined4 * __thiscall cme_hwprofile_cmehwprofile__Config__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00a78960(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_00a78960);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] cme_hwprofile_cmehwprofile__Device virtual function #7
   VTable: vtable_cme_hwprofile_cmehwprofile__Device at 01929ea8 */

undefined4 * __thiscall cme_hwprofile_cmehwprofile__Device__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00a788a0(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x34,*(int *)((int)this + -4),FUN_00a788a0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


void log4cxx_UnrealAppender__vfunc_0(int param_1)

{
  int iVar1;
  int iVar2;
  ObjectPtrT<class_log4cxx::Level> local_14 [8];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016f45f9;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar2 = log4cxx::Level::getFatal();
  iVar1 = *(int *)(iVar2 + 4);
  iVar2 = *(int *)(*(int *)(param_1 + 4) + 0x24);
  uStack_4 = 0xffffffff;
  log4cxx::helpers::ObjectPtrT<class_log4cxx::Level>::~ObjectPtrT<class_log4cxx::Level>(local_14);
  if (iVar1 != iVar2) {
    iVar2 = log4cxx::Level::getError();
    iVar1 = *(int *)(iVar2 + 4);
    iVar2 = *(int *)(*(int *)(param_1 + 4) + 0x24);
    uStack_4 = 0xffffffff;
    log4cxx::helpers::ObjectPtrT<class_log4cxx::Level>::~ObjectPtrT<class_log4cxx::Level>(local_14);
    if (iVar1 != iVar2) {
      iVar2 = log4cxx::Level::getWarn();
      iVar1 = *(int *)(iVar2 + 4);
      iVar2 = *(int *)(*(int *)(param_1 + 4) + 0x24);
      uStack_4 = 0xffffffff;
      log4cxx::helpers::ObjectPtrT<class_log4cxx::Level>::~ObjectPtrT<class_log4cxx::Level>
                (local_14);
      if (iVar1 != iVar2) {
        iVar2 = log4cxx::Level::getInfo();
        iVar1 = *(int *)(iVar2 + 4);
        iVar2 = *(int *)(*(int *)(param_1 + 4) + 0x24);
        uStack_4 = 0xffffffff;
        log4cxx::helpers::ObjectPtrT<class_log4cxx::Level>::~ObjectPtrT<class_log4cxx::Level>
                  (local_14);
        if (iVar1 != iVar2) {
          iVar2 = log4cxx::Level::getDebug();
          iVar1 = *(int *)(iVar2 + 4);
          iVar2 = *(int *)(*(int *)(param_1 + 4) + 0x24);
          uStack_4 = 0xffffffff;
          log4cxx::helpers::ObjectPtrT<class_log4cxx::Level>::~ObjectPtrT<class_log4cxx::Level>
                    (local_14);
          if (iVar1 != iVar2) {
            log4cxx::Level::getTrace();
            uStack_4 = 0xffffffff;
            log4cxx::helpers::ObjectPtrT<class_log4cxx::Level>::~ObjectPtrT<class_log4cxx::Level>
                      (local_14);
          }
        }
      }
    }
  }
  ExceptionList = pvStack_c;
  return;
}


Class * __thiscall log4cxx_UnrealAppender_ClazzUnrealAppender__vfunc_0(void *this,byte param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f4509;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  *(undefined ***)this = log4cxx::UnrealAppender::ClazzUnrealAppender::vftable;
  local_4 = 0xffffffff;
  log4cxx::helpers::Class::~Class(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvStack_c;
  return this;
}


undefined4 * __thiscall _A0xa251d603_ErrorInfoContext__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_016f91c8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this =
       CME::CountedBaseTempl<int,struct_CME::Detail::DefaultCountTypeTraits<int,int>_>::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


undefined4 * __thiscall _A0x6c617049_AbilityAction__vfunc_0(void *this,byte param_1)

{
  FUN_00e3e750(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall _A0x6c617049_ItemAction__vfunc_0(void *this,byte param_1)

{
  FUN_00e3e960(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


exception * __thiscall compositor_error__vfunc_0(void *this,byte param_1)

{
  FUN_00e9c2f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall quex_VToken___circular_queue__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  
  pvVar1 = *(void **)((int)this + 4);
  *(undefined ***)this = circular_queue<class_quex::Token>::vftable;
  if (pvVar1 != (void *)0x0) {
    _eh_vector_destructor_iterator_(pvVar1,0x24,*(int *)((int)pvVar1 + -4),FUN_00cf3a90);
                    /* WARNING: Subroutine does not return */
    scalable_free((int)pvVar1 + -4);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall quex_quex_VToken___TokenQueue__vfunc_0(void *this,byte param_1)

{
  FUN_00ea3530(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: A0x45c6b2f3_FDecalMaterial (slot 0) */

undefined4 * __thiscall _A0x45c6b2f3_FDecalMaterial__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01727b00;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = FMaterialResource::vftable;
  local_4 = 0xffffffff;
  FUN_006f6f40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


type_info * __thiscall type_info__vfunc_0(void *this,byte param_1)

{
  type_info *ptVar1;
  
  if ((param_1 & 2) == 0) {
    type_info::_type_info_dtor_internal_method(this);
    ptVar1 = this;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
  }
  else {
    ptVar1 = (type_info *)((int)this + -4);
    _eh_vector_destructor_iterator_
              (this,0xc,*(int *)ptVar1,type_info::_type_info_dtor_internal_method);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(ptVar1);
    }
  }
  return ptVar1;
}


/* WARNING: Function: __SEH_prolog4 replaced with injection: SEH_prolog4 */
/* WARNING: Function: __SEH_epilog4 replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    ___tmainCRTStartup
   
   Library: Visual Studio 2008 Release */

int ___tmainCRTStartup(void)

{
  byte bVar1;
  LONG LVar2;
  BOOL BVar3;
  uint uVar4;
  int iVar5;
  bool bVar6;
  byte *pbVar7;
  _STARTUPINFOA local_6c;
  byte *local_24;
  uint local_20;
  undefined4 uStack_c;
  undefined4 local_8;
  
  uStack_c = 0x12376ef;
  bVar6 = false;
  local_20 = 0;
  local_8 = 0;
  GetStartupInfoA(&local_6c);
  local_8 = 1;
  iVar5 = *(int *)((int)Self + 4);
  do {
    LVar2 = InterlockedCompareExchange((LONG *)&DAT_01f159e0,iVar5,0);
    if (LVar2 == 0) {
LAB_01237746:
      if (DAT_01f159dc == 1) {
        _amsg_exit(0x1f);
      }
      else if (DAT_01f159dc == 0) {
        DAT_01f159dc = 1;
        iVar5 = initterm_e(&DAT_017f692c,&DAT_017f693c);
        if (iVar5 != 0) {
          return 0xff;
        }
      }
      else {
        DAT_01f0549c = 1;
      }
      if (DAT_01f159dc == 1) {
        initterm(&DAT_017f24ec,&DAT_017f6928);
        DAT_01f159dc = 2;
      }
      if (!bVar6) {
        InterlockedExchange((LONG *)&DAT_01f159e0,0);
      }
      if ((DAT_01f159e4 != (code *)0x0) &&
         (BVar3 = __IsNonwritableInCurrentImage((PBYTE)&DAT_01f159e4), BVar3 != 0)) {
        (*DAT_01f159e4)(0,2,0);
      }
      pbVar7 = *(byte **)_acmdln_exref;
      while ((bVar1 = *pbVar7, local_24 = pbVar7, 0x20 < bVar1 || ((bVar1 != 0 && (local_20 != 0))))
            ) {
        if (bVar1 == 0x22) {
          local_20 = (uint)(local_20 == 0);
        }
        iVar5 = _ismbblead((uint)bVar1);
        if (iVar5 != 0) {
          pbVar7 = pbVar7 + 1;
        }
        pbVar7 = pbVar7 + 1;
      }
      for (; (*local_24 != 0 && (*local_24 < 0x21)); local_24 = local_24 + 1) {
      }
      if (((byte)local_6c.dwFlags & 1) == 0) {
        uVar4 = 10;
      }
      else {
        uVar4 = (uint)local_6c.wShowWindow;
      }
      DAT_01f05498 = FUN_004161d0(0x400000,0,local_24,uVar4);
      if (DAT_01f0548c != 0) {
        if (DAT_01f0549c == 0) {
          _cexit();
        }
        return DAT_01f05498;
      }
                    /* WARNING: Subroutine does not return */
      exit(DAT_01f05498);
    }
    if (LVar2 == iVar5) {
      bVar6 = true;
      goto LAB_01237746;
    }
    Sleep(1000);
  } while( true );
}


/* Library Function - Single Match
    ___security_init_cookie
   
   Library: Visual Studio 2005 Release */

void __cdecl ___security_init_cookie(void)

{
  DWORD DVar1;
  DWORD DVar2;
  DWORD DVar3;
  uint uVar4;
  LARGE_INTEGER local_14;
  _FILETIME local_c;
  
  local_c.dwLowDateTime = 0;
  local_c.dwHighDateTime = 0;
  if ((DAT_01e7f9a0 == 0xbb40e64e) || ((DAT_01e7f9a0 & 0xffff0000) == 0)) {
    GetSystemTimeAsFileTime(&local_c);
    uVar4 = local_c.dwHighDateTime ^ local_c.dwLowDateTime;
    DVar1 = GetCurrentProcessId();
    DVar2 = GetCurrentThreadId();
    DVar3 = GetTickCount();
    QueryPerformanceCounter(&local_14);
    DAT_01e7f9a0 = uVar4 ^ DVar1 ^ DVar2 ^ DVar3 ^ local_14.s.HighPart ^ local_14.s.LowPart;
    if (DAT_01e7f9a0 == 0xbb40e64e) {
      DAT_01e7f9a0 = 0xbb40e64f;
    }
    else if ((DAT_01e7f9a0 & 0xffff0000) == 0) {
      DAT_01e7f9a0 = DAT_01e7f9a0 | DAT_01e7f9a0 << 0x10;
    }
  }
  DAT_01e7f9a4 = ~DAT_01e7f9a0;
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* Library Function - Single Match
    ___report_gsfailure
   
   Libraries: Visual Studio 2005 Release, Visual Studio 2008 Release, Visual Studio 2010 Release */

void __cdecl ___report_gsfailure(void)

{
  undefined4 in_EAX;
  HANDLE hProcess;
  undefined4 in_ECX;
  undefined4 in_EDX;
  undefined4 unaff_EBX;
  undefined4 unaff_EBP;
  undefined4 unaff_ESI;
  undefined4 unaff_EDI;
  undefined2 in_ES;
  undefined2 in_CS;
  undefined2 in_SS;
  undefined2 in_DS;
  undefined2 in_FS;
  undefined2 in_GS;
  byte in_AF;
  byte in_TF;
  byte in_IF;
  byte in_NT;
  byte in_AC;
  byte in_VIF;
  byte in_VIP;
  byte in_ID;
  undefined4 unaff_retaddr;
  UINT uExitCode;
  undefined4 local_32c;
  undefined4 local_328;
  
  _DAT_01f055d8 =
       (uint)(in_NT & 1) * 0x4000 | (uint)SBORROW4((int)&stack0xfffffffc,0x328) * 0x800 |
       (uint)(in_IF & 1) * 0x200 | (uint)(in_TF & 1) * 0x100 | (uint)((int)&local_32c < 0) * 0x80 |
       (uint)(&stack0x00000000 == (undefined1 *)0x32c) * 0x40 | (uint)(in_AF & 1) * 0x10 |
       (uint)((POPCOUNT((uint)&local_32c & 0xff) & 1U) == 0) * 4 |
       (uint)(&stack0xfffffffc < (undefined1 *)0x328) | (uint)(in_ID & 1) * 0x200000 |
       (uint)(in_VIP & 1) * 0x100000 | (uint)(in_VIF & 1) * 0x80000 | (uint)(in_AC & 1) * 0x40000;
  _DAT_01f055dc = &stack0x00000004;
  _DAT_01f05518 = 0x10001;
  _DAT_01f054c0 = 0xc0000409;
  _DAT_01f054c4 = 1;
  local_32c = DAT_01e7f9a0;
  local_328 = DAT_01e7f9a4;
  _DAT_01f054cc = unaff_retaddr;
  _DAT_01f055a4 = in_GS;
  _DAT_01f055a8 = in_FS;
  _DAT_01f055ac = in_ES;
  _DAT_01f055b0 = in_DS;
  _DAT_01f055b4 = unaff_EDI;
  _DAT_01f055b8 = unaff_ESI;
  _DAT_01f055bc = unaff_EBX;
  _DAT_01f055c0 = in_EDX;
  _DAT_01f055c4 = in_ECX;
  _DAT_01f055c8 = in_EAX;
  _DAT_01f055cc = unaff_EBP;
  DAT_01f055d0 = unaff_retaddr;
  _DAT_01f055d4 = in_CS;
  _DAT_01f055e0 = in_SS;
  DAT_01f05510 = IsDebuggerPresent();
  _crt_debugger_hook(1);
  SetUnhandledExceptionFilter((LPTOP_LEVEL_EXCEPTION_FILTER)0x0);
  UnhandledExceptionFilter((_EXCEPTION_POINTERS *)&PTR_DAT_01ac8b68);
  if (DAT_01f05510 == 0) {
    _crt_debugger_hook(1);
  }
  uExitCode = 0xc0000409;
  hProcess = GetCurrentProcess();
  TerminateProcess(hProcess,uExitCode);
  return;
}


undefined4 * __thiscall _com_error__vfunc_0(void *this,byte param_1)

{
  int *piVar1;
  HANDLE hHeap;
  
  piVar1 = *(int **)((int)this + 8);
  *(undefined ***)this = _com_error::vftable;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  if (*(int *)((int)this + 0xc) != 0) {
    hHeap = GetProcessHeap();
    if (hHeap != (HANDLE)0x0) {
      HeapFree(hHeap,0,*(LPVOID *)((int)this + 0xc));
    }
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall stCamera__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = stCamera::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CMEWinCrashReport_crashapp__CrashFileList (slot 0)
   Also in vtable: cme_hwprofile_xsd__hexBinary (slot 0)
   Also in vtable: SGWUIPersistence_gui__URect (slot 0)
   Also in vtable: SGWSystemOptions_SGWSystemOptions__OptionGroupType (slot 0)
   Also in vtable: SGWBindableActions_action__BindingSaveType (slot 0) */

undefined4 CMEWinCrashReport_crashapp__CrashFileList__vfunc_0(void)

{
  return 9;
}


undefined4 * __thiscall _N___FCDPhysicsParameter__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FCDPhysicsParameter<bool>::vftable;
  if (*(int *)((int)this + 0x40) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(int *)((int)this + 0x40));
  }
  FUN_0137d7e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
_J_V__LongIntegerDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_01599d00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
_K_V__LongIntegerDataType___SimpleMetaDataType__vfunc_0(void *this,byte param_1)

{
  FUN_01599ee0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


