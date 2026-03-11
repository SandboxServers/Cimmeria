/*
 * SGW.exe Decompilation - 01_sgw_game_classes
 * Classes: 1
 * Stargate Worlds Client
 */


/* ========== SGW_Game.c ========== */

/*
 * SGW.exe - SGW_Game (519 functions)
 */

/* Also in vtable: SGWAudioDevice (slot 0) */

undefined4 * __thiscall SGWAudioDevice__vfunc_0(void *this,byte param_1)

{
  FUN_0055f810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: SGWExternalWindowManager (slot 0) */

void __thiscall SGWExternalWindowManager__vfunc_0(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(param_2);
  }
  return;
}


/* [VTable] SGWExternalWindowManager virtual function #1
   VTable: vtable_SGWExternalWindowManager at 0183f26c */

void __thiscall SGWExternalWindowManager__vfunc_1(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 0xc))(param_2);
  }
  return;
}


/* [VTable] SGWExternalWindowManager virtual function #2
   VTable: vtable_SGWExternalWindowManager at 0183f26c */

void __thiscall
SGWExternalWindowManager__vfunc_2
          (void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 0x10))(param_2,param_3);
  }
  return;
}


/* [VTable] SGWExternalWindowManager virtual function #3
   VTable: vtable_SGWExternalWindowManager at 0183f26c */

void __thiscall SGWExternalWindowManager__vfunc_3(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 0x14))(param_2);
  }
  return;
}


/* [VTable] SGWExternalWindowManager virtual function #4
   VTable: vtable_SGWExternalWindowManager at 0183f26c */

void __thiscall SGWExternalWindowManager__vfunc_4(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 0x18))(param_2);
  }
  return;
}


/* [VTable] SGWExternalWindowManager virtual function #5
   VTable: vtable_SGWExternalWindowManager at 0183f26c */

void __thiscall SGWExternalWindowManager__vfunc_5(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 0x1c))(param_2);
  }
  return;
}


/* [VTable] SGWExternalWindowManager virtual function #6
   VTable: vtable_SGWExternalWindowManager at 0183f26c */

void __thiscall SGWExternalWindowManager__vfunc_6(void *this,undefined4 param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)SGWExternalWindowManager__unknown_00568050((int)this);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 0x20))(param_2);
  }
  return;
}


/* [VTable] SGWUIManager virtual function #2
   VTable: vtable_SGWUIManager at 0183fb44 */

void __thiscall SGWUIManager__vfunc_2(void *this,int param_1,int *param_2)

{
  if (param_1 == 0x2d) {
    FUN_0056c1b0(this,param_2);
  }
  return;
}


/* Also in vtable: SGWUIManager (slot 0) */

undefined4 * __thiscall SGWUIManager__vfunc_0(void *this,byte param_1)

{
  FUN_0056ec30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: SGWUIPersistence_gui__WindowSaveType (slot 0)
   Also in vtable: SGWSystemOptions__SGWSystemOptions__optionGroups (slot 0)
   Also in vtable: SGWBindableActions__action__SavedBindings (slot 0)
   Also in vtable: SGWTableOfContents_toc__ModuleType (slot 0)
   Also in vtable: CMETextCmds_cmd__OptionalParamType (slot 0) */

undefined4 CMETextCmds_cmd__OptionalParamType__vfunc_0(void)

{
  return 0xb;
}


/* [VTable] ASGWSkyDomeActor virtual function #114
   VTable: vtable_ASGWSkyDomeActor at 0184ed9c */

void __fastcall ASGWSkyDomeActor__vfunc_114(int param_1)

{
  *(uint *)(param_1 + 0x74) = *(uint *)(param_1 + 0x74) | 0x40000000;
  return;
}


/* [VTable] ASGWSkyDomeActor virtual function #86
   VTable: vtable_ASGWSkyDomeActor at 0184ed9c */

undefined4 ASGWSkyDomeActor__vfunc_86(void)

{
  return 1;
}


/* Also in vtable: ASGWSkyDomeActor (slot 0) */

undefined4 ASGWSkyDomeActor__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWSkyDomeActor virtual function #1
   VTable: vtable_ASGWSkyDomeActor at 0184ed9c */

bool __fastcall ASGWSkyDomeActor__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee3998 == (undefined4 *)0x0) {
    DAT_01ee3998 = FUN_0076f700();
    FUN_00769330();
  }
  return puVar1 != DAT_01ee3998;
}


/* [VTable] ASGWSkyDomeActor virtual function #83
   VTable: vtable_ASGWSkyDomeActor at 0184ed9c */

void __thiscall ASGWSkyDomeActor__vfunc_83(void *this,float *param_1)

{
  int iVar1;
  
  if ((*(uint *)((int)this + 0x74) & 0x80000000) == 0) {
    iVar1 = *(int *)((int)this + 0x1b0);
    *(float *)(iVar1 + 0x240) = *(float *)(iVar1 + 0x240) + *param_1;
    *(float *)(iVar1 + 0x244) = param_1[1] + *(float *)(iVar1 + 0x244);
    *(float *)(iVar1 + 0x248) = param_1[2] + *(float *)(iVar1 + 0x248);
  }
  return;
}


/* [VTable] ASGWSkyDomeActor virtual function #106
   VTable: vtable_ASGWSkyDomeActor at 0184ed9c */

void __thiscall ASGWSkyDomeActor__vfunc_106(void *this,float param_1)

{
  int *piVar1;
  
  AActor__vfunc_106(this,param_1);
  piVar1 = *(int **)((int)this + 0x1b0);
  if ((piVar1 != (int *)0x0) && ((*(byte *)(piVar1 + 0x14) & 1) != 0)) {
    (**(code **)(*piVar1 + 0x124))(param_1);
  }
  return;
}


/* [VTable] ASGWSkyDomeActor virtual function #2
   VTable: vtable_ASGWSkyDomeActor at 0184ed9c */

int * __thiscall ASGWSkyDomeActor__vfunc_2(void *this,byte param_1)

{
  FUN_00606d00(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #188
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

void __fastcall ASGWSpecSpawnPoint__vfunc_188(undefined4 param_1,code *param_2)

{
  float fVar1;
  float fVar2;
  float fVar3;
  float fVar4;
  float fVar5;
  float fVar6;
  float fVar7;
  bool bVar8;
  int iVar9;
  int in_EAX;
  undefined8 *puVar10;
  float *pfVar11;
  uint uVar12;
  int iVar13;
  int unaff_EBX;
  int *piVar14;
  int unaff_EBP;
  void *unaff_EDI;
  float fVar15;
  int in_stack_00000010;
  int in_stack_00000014;
  int in_stack_00000018;
  float in_stack_0000001c;
  int iStack00000020;
  int in_stack_00000024;
  int in_stack_00000028;
  int in_stack_00000030;
  int in_stack_00000034;
  float in_stack_0000003c;
  float in_stack_00000040;
  float in_stack_00000044;
  float in_stack_00000048;
  float in_stack_0000004c;
  float in_stack_00000050;
  float in_stack_00000054;
  float in_stack_00000058;
  float in_stack_0000005c;
  float in_stack_00000060;
  float in_stack_00000064;
  float fStack00000068;
  float fStack0000006c;
  float fStack00000070;
  float fStack00000074;
  float in_stack_00000080;
  float in_stack_00000084;
  void *in_stack_0000008c;
  undefined4 in_stack_00000094;
  
  do {
    (*param_2)(&stack0x00000030,*(undefined4 *)(unaff_EBP + 0xc),in_EAX + unaff_EBX + 0x18,
               in_EAX + unaff_EBX + 0x14);
    while( true ) {
      if (*(int *)(unaff_EBX + 0x14 + *(int *)((int)unaff_EDI + 0xb4)) != 0) {
        **(undefined4 **)(unaff_EBP + 0x14) = 1;
        in_stack_0000001c =
             *(float *)(unaff_EBX + 0xc + *(int *)((int)unaff_EDI + 0xb4)) + in_stack_0000001c;
        in_stack_00000018 = in_stack_00000010;
      }
      while( true ) {
        iStack00000020 = 0;
        fVar15 = DAT_01814190;
        if (0 < *(int *)(*(int *)(unaff_EBP + 0xc) + 4)) {
          do {
            uVar12 = (uint)*(byte *)(**(int **)(unaff_EBP + 0xc) + iStack00000020);
            if (in_stack_00000024 == 0) {
              iVar13 = **(int **)(unaff_EBP + 8);
              iVar9 = uVar12 * 0x20;
              pfVar11 = (float *)(iVar9 + in_stack_00000030);
              if (*(float *)(iVar9 + 0xc + in_stack_00000030) * *(float *)(iVar13 + 0xc + iVar9) +
                  *(float *)(iVar9 + 8 + in_stack_00000030) * *(float *)(iVar13 + 8 + iVar9) +
                  pfVar11[1] * ((float *)(iVar13 + iVar9))[1] +
                  *pfVar11 * *(float *)(iVar13 + iVar9) < 0.0) {
                in_stack_00000058 = *pfVar11 * fVar15;
                in_stack_0000005c = pfVar11[1] * fVar15;
                *(ulonglong *)pfVar11 = CONCAT44(in_stack_0000005c,in_stack_00000058);
                in_stack_00000060 = pfVar11[2] * fVar15;
                in_stack_00000064 = pfVar11[3] * fVar15;
                *(ulonglong *)(pfVar11 + 2) = CONCAT44(in_stack_00000064,in_stack_00000060);
              }
              fVar1 = *(float *)(*(int *)((int)unaff_EDI + 0xb4) + 0xc + unaff_EBX);
              fVar15 = *(float *)(iVar9 + 0x18 + in_stack_00000030);
              fVar2 = *(float *)(iVar9 + 8 + in_stack_00000030);
              fVar3 = *(float *)(iVar9 + 0xc + in_stack_00000030);
              fVar4 = *(float *)(iVar9 + 0x1c + in_stack_00000030);
              fVar5 = *(float *)(iVar9 + in_stack_00000030);
              fVar6 = *(float *)(iVar9 + 0x10 + in_stack_00000030);
              fVar7 = *(float *)(iVar9 + 4 + in_stack_00000030);
              pfVar11 = (float *)(**(int **)(unaff_EBP + 8) + iVar9);
              pfVar11[5] = *(float *)(iVar9 + 0x14 + in_stack_00000030) * fVar1 + pfVar11[5];
              pfVar11[6] = fVar15 * fVar1 + pfVar11[6];
              pfVar11[4] = pfVar11[4] + fVar1 * fVar6;
              pfVar11[3] = fVar3 * fVar1 + pfVar11[3];
              fVar15 = DAT_01814190;
              *pfVar11 = *pfVar11 + fVar1 * fVar5;
              pfVar11[1] = fVar7 * fVar1 + pfVar11[1];
              pfVar11[2] = fVar2 * fVar1 + pfVar11[2];
              pfVar11[7] = fVar4 * fVar1 + pfVar11[7];
            }
            else {
              in_stack_00000084 = *(float *)(*(int *)((int)unaff_EDI + 0xb4) + 0xc + unaff_EBX);
              iVar13 = uVar12 * 0x20;
              in_stack_0000003c = *(float *)(iVar13 + 0x10 + in_stack_00000030) * in_stack_00000084;
              in_stack_00000040 = *(float *)(iVar13 + 0x14 + in_stack_00000030) * in_stack_00000084;
              in_stack_00000044 = *(float *)(iVar13 + 0x18 + in_stack_00000030) * in_stack_00000084;
              in_stack_00000048 = *(float *)(iVar13 + in_stack_00000030) * in_stack_00000084;
              in_stack_0000004c = *(float *)(iVar13 + 4 + in_stack_00000030) * in_stack_00000084;
              in_stack_00000050 = *(float *)(iVar13 + 8 + in_stack_00000030) * in_stack_00000084;
              puVar10 = (undefined8 *)(**(int **)(unaff_EBP + 8) + iVar13);
              in_stack_00000054 = *(float *)(iVar13 + 0xc + in_stack_00000030) * in_stack_00000084;
              in_stack_00000084 = *(float *)(iVar13 + 0x1c + in_stack_00000030) * in_stack_00000084;
              *puVar10 = CONCAT44(in_stack_0000004c,in_stack_00000048);
              *(float *)(puVar10 + 1) = in_stack_00000050;
              *(float *)((int)puVar10 + 0xc) = in_stack_00000054;
              *(float *)(puVar10 + 2) = in_stack_0000003c;
              *(float *)((int)puVar10 + 0x14) = in_stack_00000040;
              puVar10[3] = CONCAT44(in_stack_00000084,in_stack_00000044);
              in_stack_00000080 = in_stack_00000044;
            }
            if (in_stack_00000010 == in_stack_00000014) {
              FUN_004f57a0((float *)(**(int **)(unaff_EBP + 8) + uVar12 * 0x20));
              fVar15 = DAT_01814190;
            }
            iStack00000020 = iStack00000020 + 1;
          } while (iStack00000020 < *(int *)(*(int *)(unaff_EBP + 0xc) + 4));
        }
        pfVar11 = *(float **)(unaff_EBP + 0x10);
        in_stack_00000024 = 0;
        do {
          iVar13 = unaff_EBX;
          in_stack_00000010 = in_stack_00000010 + 1;
          unaff_EBX = iVar13 + 0x40;
          if (in_stack_00000014 < in_stack_00000010) {
            piVar14 = *(int **)(unaff_EBP + 0x14);
            if (*piVar14 != 0) {
              bVar8 = true;
              if (-1 < in_stack_00000018) {
                iVar13 = 0;
                in_stack_00000018 = in_stack_00000018 + 1;
                do {
                  if ((DAT_01818ca0 < *(float *)(*(int *)((int)unaff_EDI + 0xb4) + 0xc + iVar13)) &&
                     (*(int *)(*(int *)((int)unaff_EDI + 0xb4) + iVar13 + 0x14) != 0)) {
                    iVar9 = *(int *)((int)unaff_EDI + 0xb4);
                    in_stack_00000064 = *(float *)(iVar9 + 0xc + iVar13) / in_stack_0000001c;
                    in_stack_0000003c = *(float *)(iVar9 + 0x28 + iVar13) * in_stack_00000064;
                    in_stack_00000058 = *(float *)(iVar9 + iVar13 + 0x18) * in_stack_00000064;
                    in_stack_0000005c = *(float *)(iVar9 + 0x1c + iVar13) * in_stack_00000064;
                    in_stack_00000084 = *(float *)(iVar9 + 0x34 + iVar13) * in_stack_00000064;
                    in_stack_00000040 = *(float *)(iVar9 + 0x2c + iVar13) * in_stack_00000064;
                    in_stack_00000044 = *(float *)(iVar9 + 0x30 + iVar13) * in_stack_00000064;
                    in_stack_00000060 = *(float *)(iVar9 + 0x20 + iVar13) * in_stack_00000064;
                    in_stack_00000064 = *(float *)(iVar9 + 0x24 + iVar13) * in_stack_00000064;
                    _fStack00000068 = CONCAT44(in_stack_0000005c,in_stack_00000058);
                    _fStack00000070 = CONCAT44(in_stack_00000064,in_stack_00000060);
                    in_stack_00000080 = in_stack_00000044;
                    if (bVar8) {
                      *(undefined8 *)pfVar11 = _fStack00000068;
                      pfVar11[2] = in_stack_00000060;
                      pfVar11[3] = in_stack_00000064;
                      *(ulonglong *)(pfVar11 + 4) = CONCAT44(in_stack_00000040,in_stack_0000003c);
                      *(ulonglong *)(pfVar11 + 6) = CONCAT44(in_stack_00000084,in_stack_00000044);
                      bVar8 = false;
                    }
                    else {
                      if (pfVar11[3] * in_stack_00000064 + pfVar11[1] * in_stack_0000005c +
                          pfVar11[2] * in_stack_00000060 + *pfVar11 * in_stack_00000058 < 0.0) {
                        in_stack_00000048 = in_stack_00000058 * DAT_01814190;
                        in_stack_0000004c = in_stack_0000005c * DAT_01814190;
                        in_stack_00000050 = in_stack_00000060 * DAT_01814190;
                        in_stack_00000054 = in_stack_00000064 * DAT_01814190;
                        _fStack00000068 = CONCAT44(in_stack_0000004c,in_stack_00000048);
                        _fStack00000070 = CONCAT44(in_stack_00000054,in_stack_00000050);
                      }
                      pfVar11[4] = pfVar11[4] + in_stack_0000003c;
                      pfVar11[5] = in_stack_00000040 + pfVar11[5];
                      pfVar11[6] = in_stack_00000044 + pfVar11[6];
                      *pfVar11 = *pfVar11 + fStack00000068;
                      pfVar11[1] = pfVar11[1] + fStack0000006c;
                      pfVar11[2] = pfVar11[2] + fStack00000070;
                      pfVar11[3] = pfVar11[3] + fStack00000074;
                      pfVar11[7] = pfVar11[7] + in_stack_00000084;
                    }
                  }
                  iVar13 = iVar13 + 0x40;
                  in_stack_00000018 = in_stack_00000018 + -1;
                } while (in_stack_00000018 != 0);
              }
              FUN_004f57a0(pfVar11);
              piVar14 = *(int **)(unaff_EBP + 0x14);
            }
            FUN_0068a500(unaff_EDI,*(int **)(unaff_EBP + 8),(undefined8 *)pfVar11,*piVar14);
            in_stack_00000094 = 0xffffffff;
            FUN_0160efc0(&stack0x00000030);
            ExceptionList = in_stack_0000008c;
            return;
          }
        } while (*(float *)(iVar13 + 0x4c + *(int *)((int)unaff_EDI + 0xb4)) <= DAT_01818ca0);
        if (in_stack_00000034 == 0) {
          FUN_0041a950(&stack0x00000030,in_stack_00000028,0x20,8);
        }
        iVar9 = *(int *)((int)unaff_EDI + 0xb4) + unaff_EBX;
        if (*(int *)(iVar9 + 8) != 0) break;
        *(undefined8 *)(iVar9 + 0x18) = DAT_01dc4804;
        *(undefined8 *)(iVar9 + 0x20) = DAT_01dc480c;
        *(undefined8 *)(iVar9 + 0x28) = DAT_01dc4814;
        *(undefined8 *)(iVar9 + 0x30) = DAT_01dc481c;
        *(undefined4 *)(iVar13 + 0x54 + *(int *)((int)unaff_EDI + 0xb4)) = 0;
        FUN_00683ba0(&stack0x00000030,*(int **)(unaff_EBP + 0xc),
                     (int *)(*(int *)(*(int *)((int)unaff_EDI + 0x3c) + 0x294) + 0x7c));
      }
      if ((*(byte *)(iVar9 + 0x38) & 1) == 0) break;
      iVar13 = unaff_EBX + *(int *)((int)unaff_EDI + 0xb4);
      FUN_0068a5e0(unaff_EDI,&stack0x00000030,in_stack_00000010,*(int **)(unaff_EBP + 0xc),
                   (undefined8 *)(iVar13 + 0x18),(int *)(iVar13 + 0x14));
    }
    in_EAX = *(int *)((int)unaff_EDI + 0xb4);
    param_2 = *(code **)(**(int **)(in_EAX + 8 + unaff_EBX) + 0x130);
  } while( true );
}


/* [VTable] SGWAudioDevice virtual function #2
   VTable: vtable_SGWAudioDevice at 0183cfcc */

void SGWAudioDevice__vfunc_2(void)

{
  int *in_EAX;
  int *unaff_EBP;
  int unaff_ESI;
  int *unaff_EDI;
  undefined1 in_ZF;
  char *pcStack00000010;
  int *in_stack_00000018;
  undefined **ppuStack0000001c;
  void *in_stack_00000028;
  undefined4 in_stack_00000030;
  
  while( true ) {
    if ((bool)in_ZF) {
      pcStack00000010 = "scalable_malloc failed";
      std::exception::exception((exception *)&stack0x0000001c,&stack0x00000010);
      ppuStack0000001c = std::bad_alloc::vftable;
      in_stack_00000030 = CONCAT31((int3)((uint)in_stack_00000030 >> 8),0xc);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(&stack0x0000001c,(ThrowInfo *)&DAT_01d65cc8);
    }
    *in_EAX = (int)unaff_EDI;
    in_EAX[1] = (int)unaff_EDI;
    in_EAX[2] = (int)unaff_EDI;
    in_EAX[3] = (int)unaff_EDI;
    in_EAX[4] = (int)unaff_EDI;
    in_EAX[5] = (int)unaff_EDI;
    in_EAX[6] = (int)unaff_EDI;
    in_stack_00000018 = in_EAX + 7;
    *in_stack_00000018 = (int)unaff_EDI;
    in_EAX[8] = (int)unaff_EDI;
    in_EAX[9] = (int)unaff_EDI;
    in_stack_00000030 = CONCAT31((int3)((uint)in_stack_00000030 >> 8),0xc);
    *unaff_EBP = (int)in_EAX;
    unaff_ESI = unaff_ESI + 1;
    if (3 < unaff_ESI) break;
    unaff_EBP[5] = (int)unaff_EDI;
    pcStack00000010 = (char *)in_EAX;
    in_EAX = (int *)scalable_malloc(0x28);
    in_ZF = in_EAX == unaff_EDI;
    unaff_EBP = unaff_EBP + 1;
  }
  ExceptionList = in_stack_00000028;
  return;
}


/* Also in vtable: ASGW_TerrainSelStaticMeshActor (slot 0) */

undefined4 ASGW_TerrainSelStaticMeshActor__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGW_TerrainSelStaticMeshActor virtual function #1
   VTable: vtable_ASGW_TerrainSelStaticMeshActor at 01891e74 */

bool __fastcall ASGW_TerrainSelStaticMeshActor__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2e20 == (undefined4 *)0x0) {
    DAT_01ee2e20 = FUN_00600a80();
    FUN_005fd2b0();
  }
  return puVar1 != DAT_01ee2e20;
}


/* [VTable] ASGW_TerrainSelStaticMeshActor virtual function #2
   VTable: vtable_ASGW_TerrainSelStaticMeshActor at 01891e74 */

int * __thiscall ASGW_TerrainSelStaticMeshActor__vfunc_2(void *this,byte param_1)

{
  FUN_00766fb0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWRegion virtual function #191
   VTable: vtable_ASGWRegion at 018d735c */

undefined4 ASGWRegion__vfunc_191(void)

{
  return 0;
}


/* [VTable] ASGWRegion virtual function #192
   VTable: vtable_ASGWRegion at 018d735c */

undefined4 ASGWRegion__vfunc_192(void)

{
  return 0;
}


/* Also in vtable: ASGWRegion (slot 0) */

undefined4 ASGWRegion__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWRegion virtual function #1
   VTable: vtable_ASGWRegion at 018d735c */

bool __fastcall ASGWRegion__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee3998 == (undefined4 *)0x0) {
    DAT_01ee3998 = FUN_0076f700();
    FUN_00769330();
  }
  return puVar1 != DAT_01ee3998;
}


/* [VTable] ASGWRegion virtual function #2
   VTable: vtable_ASGWRegion at 018d735c */

int * __thiscall ASGWRegion__vfunc_2(void *this,byte param_1)

{
  FUN_00877ea0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecCircleRegion virtual function #187
   VTable: vtable_ASGWSpecCircleRegion at 018d7f14 */

undefined4 ASGWSpecCircleRegion__vfunc_187(void)

{
  return 0;
}


/* [VTable] ASGWSpecWayPoints virtual function #189
   VTable: vtable_ASGWSpecWayPoints at 018d8524 */

wchar_t * ASGWSpecWayPoints__vfunc_189(void)

{
  return L"WayPoint";
}


/* [VTable] ASGWSpecCircleRegion virtual function #188
   VTable: vtable_ASGWSpecCircleRegion at 018d7f14 */

wchar_t * ASGWSpecCircleRegion__vfunc_188(void)

{
  return L"Cylinder";
}


/* [VTable] ASGWSpecBoxRegion virtual function #187
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

undefined4 ASGWSpecBoxRegion__vfunc_187(void)

{
  return 0;
}


/* [VTable] ASGWSpecBoxRegion virtual function #188
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

wchar_t * ASGWSpecBoxRegion__vfunc_188(void)

{
  return L"BoundingBox";
}


/* [VTable] ASGWSpecWayPoints virtual function #188
   VTable: vtable_ASGWSpecWayPoints at 018d8524 */

wchar_t * ASGWSpecWayPoints__vfunc_188(void)

{
  return L"Points";
}


/* [VTable] ASGWSpecWayPoints virtual function #191
   VTable: vtable_ASGWSpecWayPoints at 018d8524 */

undefined4 ASGWSpecWayPoints__vfunc_191(void)

{
  return 1;
}


/* [VTable] ASGWSpecWayPoints virtual function #192
   VTable: vtable_ASGWSpecWayPoints at 018d8524 */

undefined4 ASGWSpecWayPoints__vfunc_192(void)

{
  return 0;
}


/* [VTable] ASGWSpecCircleRegion virtual function #191
   VTable: vtable_ASGWSpecCircleRegion at 018d7f14 */

undefined4 ASGWSpecCircleRegion__vfunc_191(void)

{
  return 1;
}


/* [VTable] ASGWSpecCircleRegion virtual function #192
   VTable: vtable_ASGWSpecCircleRegion at 018d7f14 */

undefined4 ASGWSpecCircleRegion__vfunc_192(void)

{
  return 1;
}


/* [VTable] ASGWSpecBoxRegion virtual function #191
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

undefined4 ASGWSpecBoxRegion__vfunc_191(void)

{
  return 0;
}


/* [VTable] ASGWSpecBoxRegion virtual function #192
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

undefined4 ASGWSpecBoxRegion__vfunc_192(void)

{
  return 1;
}


/* [VTable] ASGWSpecRegion virtual function #191
   VTable: vtable_ASGWSpecRegion at 018d7904 */

undefined4 ASGWSpecRegion__vfunc_191(void)

{
  return 0;
}


/* [VTable] ASGWSpecRegion virtual function #192
   VTable: vtable_ASGWSpecRegion at 018d7904 */

undefined4 ASGWSpecRegion__vfunc_192(void)

{
  return 1;
}


/* [VTable] ASGWSpecSpawnSet virtual function #189
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

wchar_t * ASGWSpecSpawnSet__vfunc_189(void)

{
  return L"SpawnSet";
}


/* [VTable] ASGWSpecSpawnSet virtual function #188
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

wchar_t * ASGWSpecSpawnSet__vfunc_188(void)

{
  return L"Points";
}


/* [VTable] ASGWSpecSpawnSet virtual function #191
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

undefined4 ASGWSpecSpawnSet__vfunc_191(void)

{
  return 0;
}


/* [VTable] ASGWSpecSpawnSet virtual function #192
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

undefined4 ASGWSpecSpawnSet__vfunc_192(void)

{
  return 0;
}


/* Also in vtable: ASGWSpecRegion (slot 0) */

undefined4 ASGWSpecRegion__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWSpecRegion virtual function #1
   VTable: vtable_ASGWSpecRegion at 018d7904 */

bool __fastcall ASGWSpecRegion__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4a5c == (undefined4 *)0x0) {
    DAT_01ee4a5c = FUN_00877d80();
    FUN_008778b0();
  }
  return puVar1 != DAT_01ee4a5c;
}


/* Also in vtable: ASGWSpecCircleRegion (slot 0) */

undefined4 ASGWSpecCircleRegion__vfunc_0(void)

{
  return 1;
}


/* Also in vtable: ASGWSpecBoxRegion (slot 0) */

undefined4 ASGWSpecBoxRegion__vfunc_0(void)

{
  return 1;
}


/* Also in vtable: ASGWSpecWayPoints (slot 0) */

undefined4 ASGWSpecWayPoints__vfunc_0(void)

{
  return 1;
}


/* Also in vtable: ASGWSpecSpawnSet (slot 0) */

undefined4 ASGWSpecSpawnSet__vfunc_0(void)

{
  return 0;
}


/* Also in vtable: ASGWSpecMeshRegion (slot 0) */

undefined4 ASGWSpecMeshRegion__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWSpecMeshRegion virtual function #1
   VTable: vtable_ASGWSpecMeshRegion at 018d882c */

bool __fastcall ASGWSpecMeshRegion__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee3998 == (undefined4 *)0x0) {
    DAT_01ee3998 = FUN_0076f700();
    FUN_00769330();
  }
  return puVar1 != DAT_01ee3998;
}


/* [VTable] ASGWSpecBoxRegion virtual function #84
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

void __thiscall ASGWSpecBoxRegion__vfunc_84(void *this,longlong *param_1)

{
  *param_1 = (ulonglong)*(uint *)((int)param_1 + 4) << 0x20;
  *(undefined4 *)(param_1 + 1) = 0;
  AActor__vfunc_84(this,(undefined4 *)param_1);
  return;
}


/* [VTable] ASGWSpecBoxRegion virtual function #91
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

void __fastcall ASGWSpecBoxRegion__vfunc_91(int *param_1)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  undefined4 uVar5;
  
  iVar2 = param_1[0x6c];
  if (*(int *)(iVar2 + 0x290) == 0) {
    iVar3 = *(int *)(iVar2 + 0x290);
    iVar1 = iVar3 + 4;
    *(int *)(iVar2 + 0x290) = iVar1;
    if (*(int *)(iVar2 + 0x294) < iVar1) {
      iVar4 = *(int *)(iVar2 + 0x28c);
      iVar1 = ((int)(iVar1 * 3 + (iVar1 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar1;
      *(int *)(iVar2 + 0x294) = iVar1;
      if ((iVar4 != 0) || (iVar1 != 0)) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        uVar5 = (**(code **)(*DAT_01ea5778 + 8))(iVar4,iVar1 * 0x2c,8);
        *(undefined4 *)(iVar2 + 0x28c) = uVar5;
      }
    }
    memset((void *)(iVar3 * 0x2c + *(int *)(iVar2 + 0x28c)),0,0xb0);
  }
  ASGWRegion__vfunc_91(param_1);
  return;
}


/* [VTable] ASGWSpecCircleRegion virtual function #1
   VTable: vtable_ASGWSpecCircleRegion at 018d7f14 */

bool __fastcall ASGWSpecCircleRegion__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4a60 == (undefined4 *)0x0) {
    DAT_01ee4a60 = FUN_00878850();
    FUN_008780e0();
  }
  return puVar1 != DAT_01ee4a60;
}


/* [VTable] ASGWSpecBoxRegion virtual function #1
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

bool __fastcall ASGWSpecBoxRegion__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4a60 == (undefined4 *)0x0) {
    DAT_01ee4a60 = FUN_00878850();
    FUN_008780e0();
  }
  return puVar1 != DAT_01ee4a60;
}


/* [VTable] ASGWSpecWayPoints virtual function #1
   VTable: vtable_ASGWSpecWayPoints at 018d8524 */

bool __fastcall ASGWSpecWayPoints__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4a60 == (undefined4 *)0x0) {
    DAT_01ee4a60 = FUN_00878850();
    FUN_008780e0();
  }
  return puVar1 != DAT_01ee4a60;
}


/* [VTable] ASGWSpecSpawnSet virtual function #1
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

bool __fastcall ASGWSpecSpawnSet__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4a60 == (undefined4 *)0x0) {
    DAT_01ee4a60 = FUN_00878850();
    FUN_008780e0();
  }
  return puVar1 != DAT_01ee4a60;
}


/* [VTable] ASGWSpecRegion virtual function #2
   VTable: vtable_ASGWSpecRegion at 018d7904 */

int * __thiscall ASGWSpecRegion__vfunc_2(void *this,byte param_1)

{
  FUN_00879360(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecBoxRegion virtual function #2
   VTable: vtable_ASGWSpecBoxRegion at 018d7c0c */

int * __thiscall ASGWSpecBoxRegion__vfunc_2(void *this,byte param_1)

{
  FUN_008794d0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecCircleRegion virtual function #2
   VTable: vtable_ASGWSpecCircleRegion at 018d7f14 */

int * __thiscall ASGWSpecCircleRegion__vfunc_2(void *this,byte param_1)

{
  FUN_00879590(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecSpawnSet virtual function #190
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

void __thiscall ASGWSpecSpawnSet__vfunc_190(void *this,undefined4 *param_1,undefined4 *param_2)

{
  float fVar1;
  float fVar2;
  
  fVar1 = *(float *)((int)this + 0x1d0);
  fVar2 = (float)param_2[2];
  *param_1 = *param_2;
  param_1[1] = param_2[1];
  param_1[2] = fVar2 + fVar1;
  return;
}


/* [VTable] ASGWSpecSpawnSet virtual function #2
   VTable: vtable_ASGWSpecSpawnSet at 018d821c */

int * __thiscall ASGWSpecSpawnSet__vfunc_2(void *this,byte param_1)

{
  FUN_00879680(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecWayPoints virtual function #2
   VTable: vtable_ASGWSpecWayPoints at 018d8524 */

int * __thiscall ASGWSpecWayPoints__vfunc_2(void *this,byte param_1)

{
  FUN_00879740(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecMeshRegion virtual function #2
   VTable: vtable_ASGWSpecMeshRegion at 018d882c */

int * __thiscall ASGWSpecMeshRegion__vfunc_2(void *this,byte param_1)

{
  FUN_00879810(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* Also in vtable: ASGWSpecActor (slot 0) */

undefined4 ASGWSpecActor__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWSpecActor virtual function #1
   VTable: vtable_ASGWSpecActor at 018d8b5c */

bool __fastcall ASGWSpecActor__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee3998 == (undefined4 *)0x0) {
    DAT_01ee3998 = FUN_0076f700();
    FUN_00769330();
  }
  return puVar1 != DAT_01ee3998;
}


/* [VTable] ASGWSpecActor virtual function #2
   VTable: vtable_ASGWSpecActor at 018d8b5c */

int * __thiscall ASGWSpecActor__vfunc_2(void *this,byte param_1)

{
  FUN_00879de0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #86
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

undefined4 ASGWSpecSpawnPoint__vfunc_86(void)

{
  return 1;
}


/* Also in vtable: ASGWSpecSpawnPoint (slot 0) */

undefined4 ASGWSpecSpawnPoint__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #1
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

bool __fastcall ASGWSpecSpawnPoint__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4a78 == (undefined4 *)0x0) {
    DAT_01ee4a78 = FUN_00879ca0();
    FUN_00879ab0();
  }
  return puVar1 != DAT_01ee4a78;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #90
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

void __thiscall ASGWSpecSpawnPoint__vfunc_90(void *this,int param_1)

{
  undefined4 uVar1;
  
  uVar1 = DAT_01816a8c;
  *(ulonglong *)((int)this + 0x144) = CONCAT44(DAT_01816a8c,DAT_01816a8c);
  *(undefined4 *)((int)this + 0x14c) = uVar1;
  AActor__vfunc_90(this,param_1);
  return;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #91
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

void __fastcall ASGWSpecSpawnPoint__vfunc_91(int param_1)

{
  uint uVar1;
  
  AActor__vfunc_91(param_1);
  if (DAT_01ee2e28 == (undefined4 *)0x0) {
    DAT_01ee2e28 = FUN_00601640();
    FUN_005fd570();
  }
  uVar1 = FUN_004a8e10(DAT_01ee2e28,(undefined4 *)0x0,(undefined **)L"EditorMeshes.TexPropSphere",
                       (wchar_t *)0x0,0x2002,(int *)0x0,1);
  if (uVar1 != 0) {
    FUN_005fe000(*(void **)(param_1 + 0x1cc),uVar1);
  }
  if (DAT_01ee656c == (undefined4 *)0x0) {
    DAT_01ee656c = FUN_008db460();
    FUN_008d92b0();
  }
  uVar1 = FUN_004a8e10(DAT_01ee656c,(undefined4 *)0x0,
                       (undefined **)L"EngineMaterials.PropertyColorationLitMaterial",(wchar_t *)0x0
                       ,0x2002,(int *)0x0,1);
  if (uVar1 != 0) {
    FUN_00622e10(*(void **)(param_1 + 0x1cc),0,uVar1);
  }
  return;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #19
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

void __fastcall ASGWSpecSpawnPoint__vfunc_19(int param_1)

{
  int *piVar1;
  int iVar2;
  ulonglong uVar3;
  
  iVar2 = 0;
  if (0 < *(int *)(DAT_01ee2684 + 0x20c)) {
    while( true ) {
      piVar1 = (int *)(*(int *)(DAT_01ee2684 + 0x208) + iVar2 * 8);
      uVar3 = FUN_00879c60(param_1);
      if ((*piVar1 == (int)uVar3) && (piVar1[1] == (int)(uVar3 >> 0x20))) break;
      iVar2 = iVar2 + 1;
      if (*(int *)(DAT_01ee2684 + 0x20c) <= iVar2) {
        return;
      }
    }
    FUN_00654760((void *)(DAT_01ee2684 + 0x208),iVar2,1);
  }
  return;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #98
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

void __fastcall ASGWSpecSpawnPoint__vfunc_98(int param_1)

{
  ulonglong local_8;
  
  local_8 = FUN_00879c60(param_1);
  if (local_8 != 0) {
    FUN_0087a310((void *)(DAT_01ee2684 + 0x208),(int *)&local_8);
  }
  return;
}


/* [VTable] ASGWSpecSpawnPoint virtual function #2
   VTable: vtable_ASGWSpecSpawnPoint at 018d9104 */

int * __thiscall ASGWSpecSpawnPoint__vfunc_2(void *this,byte param_1)

{
  FUN_0087a430(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] ASGWSpecCoverNode virtual function #114
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

void __fastcall ASGWSpecCoverNode__vfunc_114(int param_1)

{
  *(uint *)(param_1 + 0x74) = *(uint *)(param_1 + 0x74) | 0x40000000;
  FUN_00903bf0(*(int *)(param_1 + 0x1b0));
  ASGWSpecCoverNode__unknown_009046c0
            (*(void **)(param_1 + 0x1b0),*(undefined1 *)((int)*(void **)(param_1 + 0x1b0) + 0x2d4));
  return;
}


/* [VTable] ASGWSpecCoverNode virtual function #91
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

void __fastcall ASGWSpecCoverNode__vfunc_91(int param_1)

{
  FUN_00903bf0(*(int *)(param_1 + 0x1b0));
  ASGWSpecCoverNode__unknown_009046c0
            (*(void **)(param_1 + 0x1b0),*(undefined1 *)((int)*(void **)(param_1 + 0x1b0) + 0x2d4));
  AActor__vfunc_91(param_1);
  return;
}


/* Also in vtable: ASGWSpecCoverNode (slot 0) */

undefined4 ASGWSpecCoverNode__vfunc_0(void)

{
  return 1;
}


/* [VTable] ASGWSpecCoverNode virtual function #1
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

bool __fastcall ASGWSpecCoverNode__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee3998 == (undefined4 *)0x0) {
    DAT_01ee3998 = FUN_0076f700();
    FUN_00769330();
  }
  return puVar1 != DAT_01ee3998;
}


/* [VTable] ASGWSpecCoverNode virtual function #90
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

void __thiscall ASGWSpecCoverNode__vfunc_90(void *this,int param_1)

{
  int iVar1;
  float10 fVar2;
  
  *(undefined4 *)((int)this + 0x144) = DAT_017f7ea0;
  if (param_1 != 0) {
    fVar2 = FUN_00904710(*(void **)((int)this + 0x1b0),*(float *)((int)this + 0x14c));
    *(float *)((int)this + 0x14c) = (float)fVar2;
    *(undefined4 *)(*(int *)((int)this + 0x1b0) + 0x2d8) = *(undefined4 *)((int)this + 0x148);
    *(ulonglong *)((int)this + 0xe8) = (ulonglong)*(uint *)((int)this + 0xec) << 0x20;
    *(undefined4 *)((int)this + 0xf0) = 0;
  }
  AActor__vfunc_90(this,param_1);
  if (param_1 != 0) {
    iVar1 = *(int *)((int)this + 0x1b0);
    *(undefined8 *)(iVar1 + 0x240) = 0;
    *(undefined4 *)(iVar1 + 0x248) = 0;
    (**(code **)(**(int **)((int)this + 0x1b0) + 0x1b4))();
                    /* WARNING: Could not recover jumptable at 0x008cc489. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (**(code **)(**(int **)((int)this + 0x1b0) + 0x4c))();
    return;
  }
  return;
}


/* [VTable] ASGWSpecCoverNode virtual function #7
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

void __fastcall ASGWSpecCoverNode__vfunc_7(int *param_1)

{
  undefined4 *puVar1;
  int *piVar2;
  undefined4 *puVar3;
  undefined4 *puVar4;
  int iVar5;
  float10 fVar6;
  
  AActor__vfunc_7(param_1);
  if ((*(char *)((int)param_1 + 0x1b5) != '\x04') || ((char)param_1[0x6d] != '\x04')) {
    iVar5 = 0;
    puVar3 = DAT_01ee662c;
    puVar4 = DAT_01ee2e90;
    if (0 < param_1[0x10]) {
      do {
        if (*(int *)(param_1[0xf] + iVar5 * 4) != 0) {
          if (puVar3 == (undefined4 *)0x0) {
            DAT_01ee662c = FUN_00904810();
            FUN_00903900();
            puVar3 = DAT_01ee662c;
            puVar4 = DAT_01ee2e90;
          }
          for (puVar1 = *(undefined4 **)(*(int *)(param_1[0xf] + iVar5 * 4) + 0x34);
              puVar1 != (undefined4 *)0x0; puVar1 = (undefined4 *)puVar1[0xf]) {
            if (puVar1 == puVar3) goto LAB_008cc59d;
          }
          if (puVar3 != (undefined4 *)0x0) {
            if (puVar4 == (undefined4 *)0x0) {
              DAT_01ee2e90 = FUN_0061d000();
              FUN_00625b60();
              puVar3 = DAT_01ee662c;
              puVar4 = DAT_01ee2e90;
            }
            piVar2 = *(int **)(param_1[0xf] + iVar5 * 4);
            for (puVar1 = (undefined4 *)piVar2[0xd]; puVar1 != (undefined4 *)0x0;
                puVar1 = (undefined4 *)puVar1[0xf]) {
              if (puVar1 == puVar4) goto LAB_008cc586;
            }
            if (puVar4 == (undefined4 *)0x0) {
LAB_008cc586:
              AKActorSpawnable__unknown_006e6cf0(param_1,piVar2);
              iVar5 = iVar5 + -1;
              puVar3 = DAT_01ee662c;
              puVar4 = DAT_01ee2e90;
            }
          }
        }
LAB_008cc59d:
        iVar5 = iVar5 + 1;
      } while (iVar5 < param_1[0x10]);
    }
    *(undefined1 *)(param_1[0x6c] + 0x2d5) = *(undefined1 *)((int)param_1 + 0x1b5);
    *(char *)(param_1[0x6c] + 0x2d4) = (char)param_1[0x6d];
    *(int *)(param_1[0x6c] + 0x2d8) = param_1[0x52];
    *(undefined1 *)((int)param_1 + 0x1b5) = 4;
    *(undefined1 *)(param_1 + 0x6d) = 4;
    if ((*(byte *)(param_1[0x6c] + 0x50) & 1) == 0) {
      (**(code **)(*param_1 + 0x11c))(param_1[0x6c]);
    }
    fVar6 = ASGWSpecCoverNode__unknown_009046c0
                      ((void *)param_1[0x6c],*(undefined1 *)(param_1[0x6c] + 0x2d4));
    param_1[0x53] = (int)(float)fVar6;
    (**(code **)(*(int *)param_1[0x6c] + 0x4c))(0);
    (**(code **)(*param_1 + 0x4c))(0);
  }
  return;
}


/* [VTable] ASGWSpecCoverNode virtual function #19
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

void __thiscall ASGWSpecCoverNode__vfunc_19(void *this,void *param_1)

{
  undefined4 *puVar1;
  undefined **_Str1;
  int iVar2;
  float10 fVar3;
  int local_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bdfc8;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (param_1 != (void *)0x0) {
    ExceptionList = &pvStack_c;
    puVar1 = FUN_00498ed0(param_1,local_18);
    local_4 = 0;
    if (puVar1[1] == 0) {
      _Str1 = &PTR_017f8e10;
    }
    else {
      _Str1 = (undefined **)*puVar1;
    }
    iVar2 = _wcsicmp((wchar_t *)_Str1,L"CoverNodeComponent");
    local_4 = 0xffffffff;
    FUN_0041b420(local_18);
    if (iVar2 == 0) {
      fVar3 = ASGWSpecCoverNode__unknown_009046c0
                        (*(void **)((int)this + 0x1b0),
                         *(undefined1 *)((int)*(void **)((int)this + 0x1b0) + 0x2d4));
      *(float *)((int)this + 0x14c) = (float)fVar3;
      *(undefined4 *)((int)this + 0x148) = *(undefined4 *)(*(int *)((int)this + 0x1b0) + 0x2d8);
    }
  }
  AActor__vfunc_19(this,param_1);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] ASGWSpecCoverNode virtual function #2
   VTable: vtable_ASGWSpecCoverNode at 018e8f8c */

int * __thiscall ASGWSpecCoverNode__vfunc_2(void *this,byte param_1)

{
  FUN_008cc820(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* Also in vtable: SGWScopedUIRenderToTexture (slot 0) */

undefined4 * __thiscall SGWScopedUIRenderToTexture__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = SGWScopedUIRenderToTexture::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CMETextCmds__cmd__CommandList (slot 0)
   Also in vtable: SGWBindableActions__action__ActionGroupType_action_defaultBind (slot 0)
   Also in vtable: SGWTableOfContents_cmd__OptionalParamType (slot 0) */

undefined4 CMETextCmds__cmd__CommandList__vfunc_0(void)

{
  return 0xd;
}


/* [VTable] SGWUIPersistence_gui__WindowStateType virtual function #1
   VTable: vtable_SGWUIPersistence_gui__WindowStateType at 0196395c */

void __thiscall SGWUIPersistence_gui__WindowStateType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x20) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined4 *)((int)this + 0x14) = 0;
  *(undefined4 *)((int)this + 0x18) = 0;
  *(undefined1 *)((int)this + 0x1c) = 0;
  return;
}


/* [VTable] SGWUIPersistence_gui__URect virtual function #1
   VTable: vtable_SGWUIPersistence_gui__URect at 01963938 */

void __thiscall SGWUIPersistence_gui__URect__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] SGWUIPersistence_gui__UVector2 virtual function #1
   VTable: vtable_SGWUIPersistence_gui__UVector2 at 01963914 */

void __thiscall SGWUIPersistence_gui__UVector2__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #1
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

void __thiscall SGWUIPersistence_gui__UDim__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #2
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

void SGWUIPersistence_gui__UDim__vfunc_2(void)

{
  return;
}


/* [VTable] SGWUIPersistence_gui__WindowStateType virtual function #2
   VTable: vtable_SGWUIPersistence_gui__WindowStateType at 0196395c */

void __thiscall SGWUIPersistence_gui__WindowStateType__vfunc_2(void *this,int param_1)

{
  int iVar1;
  
  FUN_00a3b030(param_1,*(uint *)((int)this + 4),0x12);
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 0x10),9);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 0x10) + 8))(param_1);
  }
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 0x14),8);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 0x14) + 8))(param_1);
  }
  return;
}


/* [VTable] SGWUIPersistence_gui__URect virtual function #2
   VTable: vtable_SGWUIPersistence_gui__URect at 01963938 */

void __thiscall SGWUIPersistence_gui__URect__vfunc_2(void *this,int param_1)

{
  int iVar1;
  
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 4),8);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 4) + 8))(param_1);
  }
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 8),8);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 8) + 8))(param_1);
  }
  return;
}


/* [VTable] SGWUIPersistence_gui__UVector2 virtual function #2
   VTable: vtable_SGWUIPersistence_gui__UVector2 at 01963914 */

void __thiscall SGWUIPersistence_gui__UVector2__vfunc_2(void *this,int param_1)

{
  int iVar1;
  
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 4),7);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 4) + 8))(param_1);
  }
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 8),7);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 8) + 8))(param_1);
  }
  return;
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #4
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

void __thiscall
SGWUIPersistence_gui__UDim__vfunc_4(void *this,uint param_1,char *param_2,int param_3,byte *param_4)

{
  FUN_00affd00(param_1,param_2,param_3,(uint)this,param_4);
  return;
}


/* [VTable] SGWUIPersistence_gui__URect virtual function #3
   VTable: vtable_SGWUIPersistence_gui__URect at 01963938 */

undefined4 __thiscall
SGWUIPersistence_gui__URect__vfunc_3(void *this,uint param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  undefined4 uVar2;
  
  iVar1 = FUN_00a40f40(param_1,(uint)this,(uint *)0x0,0,param_2,9);
  iVar1 = (**(code **)(*(int *)this + 0x10))(param_1,param_2,iVar1,param_3);
  if (iVar1 != 0) {
    return *(undefined4 *)(param_1 + 0x16138);
  }
  uVar2 = SGWUIPersistence_gui__unknown_00b00930(param_1);
  return uVar2;
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #3
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

undefined4 __thiscall
SGWUIPersistence_gui__UDim__vfunc_3(void *this,uint param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  undefined4 uVar2;
  
  iVar1 = FUN_00a40f40(param_1,(uint)this,(uint *)0x0,0,param_2,7);
  iVar1 = (**(code **)(*(int *)this + 0x10))(param_1,param_2,iVar1,param_3);
  if (iVar1 != 0) {
    return *(undefined4 *)(param_1 + 0x16138);
  }
  uVar2 = SGWUIPersistence_gui__unknown_00b00930(param_1);
  return uVar2;
}


/* [VTable] SGWUIPersistence_gui__WindowSaveType virtual function #2
   VTable: vtable_SGWUIPersistence_gui__WindowSaveType at 0183fb04 */

void __thiscall SGWUIPersistence_gui__WindowSaveType__vfunc_2(void *this,int param_1)

{
  FUN_00b00fd0(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWUIPersistence_gui__WindowSaveType virtual function #1
   VTable: vtable_SGWUIPersistence_gui__WindowSaveType at 0183fb04 */

void __thiscall SGWUIPersistence_gui__WindowSaveType__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWUIPersistence_gui__URect virtual function #6
   VTable: vtable_SGWUIPersistence_gui__URect at 01963938 */

void __thiscall
SGWUIPersistence_gui__URect__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  FUN_00b024a0(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWUIPersistence_gui__UVector2 virtual function #6
   VTable: vtable_SGWUIPersistence_gui__UVector2 at 01963914 */

void __thiscall
SGWUIPersistence_gui__UVector2__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  FUN_00b02690(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #6
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

void __thiscall
SGWUIPersistence_gui__UDim__vfunc_6(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  FUN_00b02880(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWUIPersistence_gui__URect virtual function #5
   VTable: vtable_SGWUIPersistence_gui__URect at 01963938 */

int * __thiscall
SGWUIPersistence_gui__URect__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = FUN_00b024a0(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWUIPersistence_gui__unknown_00b03590(param_1);
  }
  return piVar1;
}


/* [VTable] SGWUIPersistence_gui__UVector2 virtual function #5
   VTable: vtable_SGWUIPersistence_gui__UVector2 at 01963914 */

int * __thiscall
SGWUIPersistence_gui__UVector2__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = FUN_00b02690(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWUIPersistence_gui__unknown_00b03590(param_1);
  }
  return piVar1;
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #5
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

int * __thiscall
SGWUIPersistence_gui__UDim__vfunc_5(void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = FUN_00b02880(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWUIPersistence_gui__unknown_00b03590(param_1);
  }
  return piVar1;
}


/* [VTable] SGWUIPersistence_gui__WindowStateType virtual function #7
   VTable: vtable_SGWUIPersistence_gui__WindowStateType at 0196395c */

undefined4 * __thiscall SGWUIPersistence_gui__WindowStateType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWUIPersistence::gui__WindowStateType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x24,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00aff7d0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWUIPersistence_gui__URect virtual function #7
   VTable: vtable_SGWUIPersistence_gui__URect at 01963938 */

undefined4 * __thiscall SGWUIPersistence_gui__URect__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWUIPersistence::gui__URect::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00aff790);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWUIPersistence_gui__UVector2 virtual function #7
   VTable: vtable_SGWUIPersistence_gui__UVector2 at 01963914 */

undefined4 * __thiscall SGWUIPersistence_gui__UVector2__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWUIPersistence::gui__UVector2::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00aff760);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWUIPersistence_gui__UDim virtual function #7
   VTable: vtable_SGWUIPersistence_gui__UDim at 019638f0 */

undefined4 * __thiscall SGWUIPersistence_gui__UDim__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWUIPersistence::gui__UDim::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00aff720);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWUIPersistence_gui__WindowSaveType virtual function #7
   VTable: vtable_SGWUIPersistence_gui__WindowSaveType at 0183fb04 */

undefined4 * __thiscall SGWUIPersistence_gui__WindowSaveType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00571900(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x1c,*(int *)((int)this + -4),FUN_00571900);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionSaveType virtual function #1
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionSaveType at 019644b4 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionSaveType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x1c) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined4 *)((int)this + 0x14) = 0;
  *(undefined4 *)((int)this + 0x18) = 0;
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionSaveType virtual function #2
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionSaveType at 019644b4 */

void SGWSystemOptions_SGWSystemOptions__OptionSaveType__vfunc_2(void)

{
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionGroupType virtual function #1
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionGroupType at 01964490 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionGroupType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x1c) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined4 *)((int)this + 0x14) = 0;
  *(undefined1 *)((int)this + 0x18) = 0;
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionGroupType virtual function #2
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionGroupType at 01964490 */

void __thiscall SGWSystemOptions_SGWSystemOptions__OptionGroupType__vfunc_2(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  if ((*(int *)((int)this + 8) != 0) && (iVar1 = 0, 0 < *(int *)((int)this + 4))) {
    iVar2 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 8) + iVar2,0x14);
      (**(code **)(*(int *)(*(int *)((int)this + 8) + iVar2) + 8))(param_1);
      iVar1 = iVar1 + 1;
      iVar2 = iVar2 + 0x60;
    } while (iVar1 < *(int *)((int)this + 4));
  }
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionValueType virtual function #1
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionValueType at 0196446c */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionValueType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x14) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionValueType virtual function #2
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionValueType at 0196446c */

void SGWSystemOptions_SGWSystemOptions__OptionValueType__vfunc_2(void)

{
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionDependency virtual function #1
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionDependency at 01964448 */

void __thiscall
SGWSystemOptions_SGWSystemOptions__OptionDependency__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionDependency virtual function #2
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionDependency at 01964448 */

void SGWSystemOptions_SGWSystemOptions__OptionDependency__vfunc_2(void)

{
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__OptionGroupType_option virtual function #2
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__OptionGroupType_option at 019644d8 */

void __thiscall
SGWSystemOptions__SGWSystemOptions__OptionGroupType_option__vfunc_2(void *this,int param_1)

{
  FUN_00b06360(param_1,(int)this + 4);
  FUN_00b06240(param_1,(int)this + 0x14);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__savedOptions virtual function #2
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__savedOptions at 0183fe80 */

void __thiscall SGWSystemOptions__SGWSystemOptions__savedOptions__vfunc_2(void *this,int param_1)

{
  FUN_00b06000(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__optionGroups virtual function #2
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__optionGroups at 0183fe5c */

void __thiscall SGWSystemOptions__SGWSystemOptions__optionGroups__vfunc_2(void *this,int param_1)

{
  FUN_00b06120(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__OptionGroupType_option virtual function #1
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__OptionGroupType_option at 019644d8 */

void __fastcall SGWSystemOptions__SGWSystemOptions__OptionGroupType_option__vfunc_1(int param_1)

{
  void *pvVar1;
  void *pvVar2;
  void *pvVar3;
  undefined4 uVar4;
  int local_8 [2];
  
  pvVar2 = *(void **)(param_1 + 0xc);
  pvVar1 = (void *)(param_1 + 4);
  if (pvVar2 < *(void **)(param_1 + 8)) {
    _invalid_parameter_noinfo();
  }
  pvVar3 = *(void **)(param_1 + 8);
  if (*(void **)(param_1 + 0xc) < pvVar3) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar3,(int)pvVar1,pvVar2);
  pvVar2 = *(void **)(param_1 + 0x1c);
  pvVar1 = (void *)(param_1 + 0x14);
  if (pvVar2 < *(void **)(param_1 + 0x18)) {
    _invalid_parameter_noinfo();
  }
  pvVar3 = *(void **)(param_1 + 0x18);
  if (*(void **)(param_1 + 0x1c) < pvVar3) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar3,(int)pvVar1,pvVar2);
  *(undefined4 *)(param_1 + 0x3c) = 0;
  uVar4 = DAT_017f7ea0;
  *(undefined4 *)(param_1 + 0x24) = 0;
  *(undefined4 *)(param_1 + 0x28) = 0;
  *(undefined4 *)(param_1 + 0x2c) = 0;
  *(undefined4 *)(param_1 + 0x30) = 0;
  *(undefined1 *)(param_1 + 0x34) = 0;
  *(undefined4 *)(param_1 + 0x38) = 0;
  *(undefined4 *)(param_1 + 0x40) = uVar4;
  *(undefined4 *)(param_1 + 0x44) = uVar4;
  *(undefined1 *)(param_1 + 0x48) = 1;
  *(undefined4 *)(param_1 + 0x4c) = 0;
  *(undefined4 *)(param_1 + 0x50) = 0;
  *(undefined1 *)(param_1 + 0x54) = 0;
  *(undefined4 *)(param_1 + 0x58) = 0;
  *(undefined4 *)(param_1 + 0x5c) = 0;
  return;
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__savedOptions virtual function #1
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__savedOptions at 0183fe80 */

void __thiscall
SGWSystemOptions__SGWSystemOptions__savedOptions__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWSystemOptions__SGWSystemOptions__optionGroups virtual function #1
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__optionGroups at 0183fe5c */

void __thiscall
SGWSystemOptions__SGWSystemOptions__optionGroups__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionSaveType virtual function #7
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionSaveType at 019644b4 */

undefined4 * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionSaveType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWSystemOptions::SGWSystemOptions__OptionSaveType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x20,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b03d60);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionGroupType virtual function #7
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionGroupType at 01964490 */

undefined4 * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionGroupType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWSystemOptions::SGWSystemOptions__OptionGroupType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x20,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b03d30);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionValueType virtual function #7
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionValueType at 0196446c */

undefined4 * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionValueType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWSystemOptions::SGWSystemOptions__OptionValueType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x18,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b03d00);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions_SGWSystemOptions__OptionDependency virtual function #7
   VTable: vtable_SGWSystemOptions_SGWSystemOptions__OptionDependency at 01964448 */

undefined4 * __thiscall
SGWSystemOptions_SGWSystemOptions__OptionDependency__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWSystemOptions::SGWSystemOptions__OptionDependency::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b03cd0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__OptionGroupType_option virtual function #7
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__OptionGroupType_option at 019644d8 */

undefined4 * __thiscall
SGWSystemOptions__SGWSystemOptions__OptionGroupType_option__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    CME_UIScreen__unknown_00b0b270(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x60,*(int *)((int)this + -4),CME_UIScreen__unknown_00b0b270)
  ;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__savedOptions virtual function #7
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__savedOptions at 0183fe80 */

undefined4 * __thiscall
SGWSystemOptions__SGWSystemOptions__savedOptions__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00575f90(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_00575f90);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWSystemOptions__SGWSystemOptions__optionGroups virtual function #7
   VTable: vtable_SGWSystemOptions__SGWSystemOptions__optionGroups at 0183fe5c */

undefined4 * __thiscall
SGWSystemOptions__SGWSystemOptions__optionGroups__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00575ee0(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_00575ee0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action_defaultBind virtual function #1
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action_defaultBind at 01964be4 */

void __fastcall SGWBindableActions__action__ActionGroupType_action_defaultBind__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined1 *)(param_1 + 8) = 0;
  *(undefined1 *)(param_1 + 9) = 0;
  *(undefined1 *)(param_1 + 10) = 0;
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action_defaultBind virtual function #2
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action_defaultBind at 01964be4 */

void SGWBindableActions__action__ActionGroupType_action_defaultBind__vfunc_2(void)

{
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action virtual function #1
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action at 01964c08 */

void __fastcall SGWBindableActions__action__ActionGroupType_action__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 8) = 0;
  *(undefined4 *)(param_1 + 0xc) = 0;
  *(undefined4 *)(param_1 + 0x10) = 0;
  *(undefined4 *)(param_1 + 0x14) = 0;
  *(undefined4 *)(param_1 + 0x18) = 0;
  *(undefined1 *)(param_1 + 0x1c) = 1;
  *(undefined1 *)(param_1 + 0x1d) = 0;
  *(undefined1 *)(param_1 + 0x1e) = 0;
  *(undefined4 *)(param_1 + 0x20) = 0;
  *(undefined4 *)(param_1 + 0x24) = 0;
  *(undefined4 *)(param_1 + 0x28) = 0;
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action virtual function #2
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action at 01964c08 */

void __thiscall SGWBindableActions__action__ActionGroupType_action__vfunc_2(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  if ((*(int *)((int)this + 8) != 0) && (iVar1 = 0, 0 < *(int *)((int)this + 4))) {
    iVar2 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 8) + iVar2,0xd);
      (**(code **)(*(int *)(*(int *)((int)this + 8) + iVar2) + 8))(param_1);
      iVar1 = iVar1 + 1;
      iVar2 = iVar2 + 0xc;
    } while (iVar1 < *(int *)((int)this + 4));
  }
  return;
}


/* [VTable] SGWBindableActions_action__KeyBindType virtual function #1
   VTable: vtable_SGWBindableActions_action__KeyBindType at 01964c50 */

void __thiscall SGWBindableActions_action__KeyBindType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined1 *)((int)this + 8) = 0;
  *(undefined1 *)((int)this + 9) = 0;
  *(undefined1 *)((int)this + 10) = 0;
  return;
}


/* [VTable] SGWBindableActions_action__KeyBindType virtual function #2
   VTable: vtable_SGWBindableActions_action__KeyBindType at 01964c50 */

void SGWBindableActions_action__KeyBindType__vfunc_2(void)

{
  return;
}


/* [VTable] SGWBindableActions_action__ActionGroupType virtual function #1
   VTable: vtable_SGWBindableActions_action__ActionGroupType at 01964c2c */

void __thiscall SGWBindableActions_action__ActionGroupType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x18) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined1 *)((int)this + 0x14) = 1;
  return;
}


/* [VTable] SGWBindableActions_action__ActionGroupType virtual function #2
   VTable: vtable_SGWBindableActions_action__ActionGroupType at 01964c2c */

void __thiscall SGWBindableActions_action__ActionGroupType__vfunc_2(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  if ((*(int *)((int)this + 8) != 0) && (iVar1 = 0, 0 < *(int *)((int)this + 4))) {
    iVar2 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 8) + iVar2,0xc);
      (**(code **)(*(int *)(*(int *)((int)this + 8) + iVar2) + 8))(param_1);
      iVar1 = iVar1 + 1;
      iVar2 = iVar2 + 0x2c;
    } while (iVar1 < *(int *)((int)this + 4));
  }
  return;
}


/* [VTable] SGWBindableActions__action__SavedBindings virtual function #2
   VTable: vtable_SGWBindableActions__action__SavedBindings at 01840734 */

void __thiscall SGWBindableActions__action__SavedBindings__vfunc_2(void *this,int param_1)

{
  FUN_00b0d140(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWBindableActions__action__actionGroups virtual function #2
   VTable: vtable_SGWBindableActions__action__actionGroups at 01840710 */

void __thiscall SGWBindableActions__action__actionGroups__vfunc_2(void *this,int param_1)

{
  FUN_00b0d260(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWBindableActions_action__BindingSaveType virtual function #2
   VTable: vtable_SGWBindableActions_action__BindingSaveType at 01964c74 */

void __thiscall SGWBindableActions_action__BindingSaveType__vfunc_2(void *this,int param_1)

{
  FUN_00b0d380(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWBindableActions__action__SavedBindings virtual function #1
   VTable: vtable_SGWBindableActions__action__SavedBindings at 01840734 */

void __thiscall SGWBindableActions__action__SavedBindings__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWBindableActions__action__actionGroups virtual function #1
   VTable: vtable_SGWBindableActions__action__actionGroups at 01840710 */

void __thiscall SGWBindableActions__action__actionGroups__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWBindableActions_action__BindingSaveType virtual function #1
   VTable: vtable_SGWBindableActions_action__BindingSaveType at 01964c74 */

void __thiscall SGWBindableActions_action__BindingSaveType__vfunc_1(void *this,undefined4 param_1)

{
  void *this_00;
  void *pvVar1;
  void *pvVar2;
  int local_8 [2];
  
  this_00 = (void *)((int)this + 4);
  *(undefined4 *)((int)this + 0x1c) = param_1;
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
  *(undefined4 *)((int)this + 0x18) = 0;
  return;
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action_defaultBind virtual function #7
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action_defaultBind at 01964be4 */

undefined4 * __thiscall
SGWBindableActions__action__ActionGroupType_action_defaultBind__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWBindableActions::_action__ActionGroupType_action_defaultBind::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0xc,*(int *)((int)this + -4),FUN_00b0b4c0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions__action__ActionGroupType_action virtual function #7
   VTable: vtable_SGWBindableActions__action__ActionGroupType_action at 01964c08 */

undefined4 * __thiscall
SGWBindableActions__action__ActionGroupType_action__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWBindableActions::_action__ActionGroupType_action::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x2c,*(int *)((int)this + -4),FUN_00b0b510);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions_action__KeyBindType virtual function #7
   VTable: vtable_SGWBindableActions_action__KeyBindType at 01964c50 */

undefined4 * __thiscall SGWBindableActions_action__KeyBindType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWBindableActions::action__KeyBindType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b0b570);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions_action__ActionGroupType virtual function #7
   VTable: vtable_SGWBindableActions_action__ActionGroupType at 01964c2c */

undefined4 * __thiscall SGWBindableActions_action__ActionGroupType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWBindableActions::action__ActionGroupType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x1c,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b0b540);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions__action__SavedBindings virtual function #7
   VTable: vtable_SGWBindableActions__action__SavedBindings at 01840734 */

undefined4 * __thiscall SGWBindableActions__action__SavedBindings__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_0057d8b0(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_0057d8b0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions__action__actionGroups virtual function #7
   VTable: vtable_SGWBindableActions__action__actionGroups at 01840710 */

undefined4 * __thiscall SGWBindableActions__action__actionGroups__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_0057d7f0(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_0057d7f0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWBindableActions_action__BindingSaveType virtual function #7
   VTable: vtable_SGWBindableActions_action__BindingSaveType at 01964c74 */

undefined4 * __thiscall SGWBindableActions_action__BindingSaveType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00b11890(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x20,*(int *)((int)this + -4),FUN_00b11890);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action_defaultBind virtual function #1
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action_defaultBind at 01965cec */

void __fastcall SGWTableOfContents__action__ActionGroupType_action_defaultBind__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined1 *)(param_1 + 8) = 0;
  *(undefined1 *)(param_1 + 9) = 0;
  *(undefined1 *)(param_1 + 10) = 0;
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action_defaultBind virtual function #2
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action_defaultBind at 01965cec */

void SGWTableOfContents__action__ActionGroupType_action_defaultBind__vfunc_2(void)

{
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action virtual function #1
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action at 01965d10 */

void __fastcall SGWTableOfContents__action__ActionGroupType_action__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 8) = 0;
  *(undefined4 *)(param_1 + 0xc) = 0;
  *(undefined4 *)(param_1 + 0x10) = 0;
  *(undefined4 *)(param_1 + 0x14) = 0;
  *(undefined4 *)(param_1 + 0x18) = 0;
  *(undefined1 *)(param_1 + 0x1c) = 1;
  *(undefined1 *)(param_1 + 0x1d) = 0;
  *(undefined1 *)(param_1 + 0x1e) = 0;
  *(undefined4 *)(param_1 + 0x20) = 0;
  *(undefined4 *)(param_1 + 0x24) = 0;
  *(undefined4 *)(param_1 + 0x28) = 0;
  return;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action virtual function #2
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action at 01965d10 */

void __thiscall SGWTableOfContents__action__ActionGroupType_action__vfunc_2(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  if ((*(int *)((int)this + 8) != 0) && (iVar1 = 0, 0 < *(int *)((int)this + 4))) {
    iVar2 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 8) + iVar2,0x2a);
      (**(code **)(*(int *)(*(int *)((int)this + 8) + iVar2) + 8))(param_1);
      iVar1 = iVar1 + 1;
      iVar2 = iVar2 + 0xc;
    } while (iVar1 < *(int *)((int)this + 4));
  }
  return;
}


/* [VTable] SGWTableOfContents___cmd__CommandList_sequence virtual function #1
   VTable: vtable_SGWTableOfContents___cmd__CommandList_sequence at 01965ca4 */

void __fastcall SGWTableOfContents___cmd__CommandList_sequence__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  return;
}


/* [VTable] SGWTableOfContents__toc__ModuleType_Metadata virtual function #1
   VTable: vtable_SGWTableOfContents__toc__ModuleType_Metadata at 01965c38 */

void __fastcall SGWTableOfContents__toc__ModuleType_Metadata__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 8) = 0;
  return;
}


/* [VTable] SGWTableOfContents__toc__ModuleType_LayoutInstance virtual function #1
   VTable: vtable_SGWTableOfContents__toc__ModuleType_LayoutInstance at 01965c14 */

void __fastcall SGWTableOfContents__toc__ModuleType_LayoutInstance__vfunc_1(int param_1)

{
  *(undefined4 *)(param_1 + 4) = 0;
  *(undefined4 *)(param_1 + 8) = 0;
  return;
}


/* [VTable] SGWTableOfContents_action__KeyBindType virtual function #1
   VTable: vtable_SGWTableOfContents_action__KeyBindType at 01965d58 */

void __thiscall SGWTableOfContents_action__KeyBindType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined1 *)((int)this + 8) = 0;
  *(undefined1 *)((int)this + 9) = 0;
  *(undefined1 *)((int)this + 10) = 0;
  return;
}


/* [VTable] SGWTableOfContents_action__KeyBindType virtual function #2
   VTable: vtable_SGWTableOfContents_action__KeyBindType at 01965d58 */

void SGWTableOfContents_action__KeyBindType__vfunc_2(void)

{
  return;
}


/* [VTable] SGWTableOfContents_action__ActionGroupType virtual function #1
   VTable: vtable_SGWTableOfContents_action__ActionGroupType at 01965d34 */

void __thiscall SGWTableOfContents_action__ActionGroupType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x18) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined1 *)((int)this + 0x14) = 1;
  return;
}


/* [VTable] SGWTableOfContents_action__ActionGroupType virtual function #2
   VTable: vtable_SGWTableOfContents_action__ActionGroupType at 01965d34 */

void __thiscall SGWTableOfContents_action__ActionGroupType__vfunc_2(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  if ((*(int *)((int)this + 8) != 0) && (iVar1 = 0, 0 < *(int *)((int)this + 4))) {
    iVar2 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 8) + iVar2,0x29);
      (**(code **)(*(int *)(*(int *)((int)this + 8) + iVar2) + 8))(param_1);
      iVar1 = iVar1 + 1;
      iVar2 = iVar2 + 0x2c;
    } while (iVar1 < *(int *)((int)this + 4));
  }
  return;
}


/* [VTable] SGWTableOfContents__cmd__CommandList virtual function #1
   VTable: vtable_SGWTableOfContents__cmd__CommandList at 01965cc8 */

void __thiscall SGWTableOfContents__cmd__CommandList__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0xc) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  return;
}


/* [VTable] SGWTableOfContents__cmd__CommandList virtual function #2
   VTable: vtable_SGWTableOfContents__cmd__CommandList at 01965cc8 */

void __thiscall SGWTableOfContents__cmd__CommandList__vfunc_2(void *this,undefined4 param_1)

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


/* [VTable] SGWTableOfContents_cmd__OptionalParamType virtual function #1
   VTable: vtable_SGWTableOfContents_cmd__OptionalParamType at 01965c80 */

void __thiscall SGWTableOfContents_cmd__OptionalParamType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x10) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  return;
}


/* [VTable] SGWTableOfContents_cmd__OptionalParamType virtual function #2
   VTable: vtable_SGWTableOfContents_cmd__OptionalParamType at 01965c80 */

void SGWTableOfContents_cmd__OptionalParamType__vfunc_2(void)

{
  return;
}


/* [VTable] SGWTableOfContents_cmd__MandatoryParamType virtual function #1
   VTable: vtable_SGWTableOfContents_cmd__MandatoryParamType at 01965c5c */

void __thiscall SGWTableOfContents_cmd__MandatoryParamType__vfunc_1(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x10) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  return;
}


/* [VTable] SGWTableOfContents_cmd__MandatoryParamType virtual function #2
   VTable: vtable_SGWTableOfContents_cmd__MandatoryParamType at 01965c5c */

void SGWTableOfContents_cmd__MandatoryParamType__vfunc_2(void)

{
  return;
}


/* [VTable] SGWTableOfContents___cmd__CommandList_sequence virtual function #2
   VTable: vtable_SGWTableOfContents___cmd__CommandList_sequence at 01965ca4 */

void __thiscall SGWTableOfContents___cmd__CommandList_sequence__vfunc_2(void *this,int param_1)

{
  int iVar1;
  
  iVar1 = FUN_00a3b030(param_1,*(uint *)((int)this + 4),0xe);
  if (iVar1 == 0) {
    (**(code **)(**(int **)((int)this + 4) + 8))(param_1);
  }
  return;
}


/* [VTable] SGWTableOfContents__toc__ModuleType_Metadata virtual function #2
   VTable: vtable_SGWTableOfContents__toc__ModuleType_Metadata at 01965c38 */

void __thiscall SGWTableOfContents__toc__ModuleType_Metadata__vfunc_2(void *this,int param_1)

{
  FUN_00a3b030(param_1,*(uint *)((int)this + 4),8);
  FUN_00a3b030(param_1,*(uint *)((int)this + 8),8);
  return;
}


/* [VTable] SGWTableOfContents__toc__ModuleType_LayoutInstance virtual function #2
   VTable: vtable_SGWTableOfContents__toc__ModuleType_LayoutInstance at 01965c14 */

void __thiscall SGWTableOfContents__toc__ModuleType_LayoutInstance__vfunc_2(void *this,int param_1)

{
  FUN_00a3b030(param_1,*(uint *)((int)this + 4),8);
  return;
}


/* [VTable] SGWTableOfContents___cmd__CommandList_sequence virtual function #4
   VTable: vtable_SGWTableOfContents___cmd__CommandList_sequence at 01965ca4 */

undefined4 __thiscall
SGWTableOfContents___cmd__CommandList_sequence__vfunc_4(void *this,uint param_1)

{
  int iVar1;
  
  iVar1 = CME_UIScreen__unknown_00a4b620
                    (param_1,"Command",-1,*(uint *)((int)this + 4),(uint *)0x0,0,&DAT_01964cb2,0xe);
  if (-1 < iVar1) {
    (**(code **)(**(int **)((int)this + 4) + 0x10))(param_1,"Command",iVar1,&DAT_01964cb2);
  }
  return 0;
}


/* [VTable] SGWTableOfContents__action__SavedBindings virtual function #2
   VTable: vtable_SGWTableOfContents__action__SavedBindings at 01965de8 */

void __thiscall SGWTableOfContents__action__SavedBindings__vfunc_2(void *this,int param_1)

{
  FUN_00b14dc0(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWTableOfContents__action__actionGroups virtual function #2
   VTable: vtable_SGWTableOfContents__action__actionGroups at 01965dc4 */

void __thiscall SGWTableOfContents__action__actionGroups__vfunc_2(void *this,int param_1)

{
  FUN_00b15320(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWTableOfContents_action__BindingSaveType virtual function #2
   VTable: vtable_SGWTableOfContents_action__BindingSaveType at 01965da0 */

void __thiscall SGWTableOfContents_action__BindingSaveType__vfunc_2(void *this,int param_1)

{
  FUN_00b14ee0(param_1,(int)this + 4);
  return;
}


/* [VTable] SGWTableOfContents_cmd__CommandType virtual function #2
   VTable: vtable_SGWTableOfContents_cmd__CommandType at 01965d7c */

void __thiscall SGWTableOfContents_cmd__CommandType__vfunc_2(void *this,int param_1)

{
  FUN_00b15120(param_1,(int)this + 4);
  FUN_00b15000(param_1,(int)this + 0x14);
  return;
}


/* [VTable] SGWTableOfContents_toc__ModuleType virtual function #2
   VTable: vtable_SGWTableOfContents_toc__ModuleType at 018fa254 */

void __thiscall SGWTableOfContents_toc__ModuleType__vfunc_2(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  FUN_00a3b030(param_1,*(uint *)((int)this + 0xc),8);
  FUN_00a3b030(param_1,*(uint *)((int)this + 0x10),8);
  SGWTableOfContents_toc__unknown_00b15560(param_1,(int)this + 0x18);
  SGWTableOfContents_toc__unknown_00b15560(param_1,(int)this + 0x28);
  SGWTableOfContents_toc__unknown_00b15560(param_1,(int)this + 0x38);
  if ((*(int *)((int)this + 0x4c) != 0) && (iVar2 = 0, 0 < *(int *)((int)this + 0x48))) {
    iVar1 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 0x4c) + iVar1,0x18);
      (**(code **)(*(int *)(iVar1 + *(int *)((int)this + 0x4c)) + 8))(param_1);
      iVar2 = iVar2 + 1;
      iVar1 = iVar1 + 0xc;
    } while (iVar2 < *(int *)((int)this + 0x48));
  }
  if ((*(int *)((int)this + 0x54) != 0) && (iVar2 = 0, 0 < *(int *)((int)this + 0x50))) {
    iVar1 = 0;
    do {
      FUN_00a3b000(param_1,*(int *)((int)this + 0x54) + iVar1,0x1a);
      (**(code **)(*(int *)(iVar1 + *(int *)((int)this + 0x54)) + 8))(param_1);
      iVar2 = iVar2 + 1;
      iVar1 = iVar1 + 0xc;
    } while (iVar2 < *(int *)((int)this + 0x50));
  }
  FUN_00b15440(param_1,(int)this + 0x58);
  FUN_00b15320(param_1,(int)this + 0x68);
  FUN_00b15240(param_1,(int)this + 0x78);
  FUN_00b15240(param_1,(int)this + 0x88);
  return;
}


/* [VTable] SGWTableOfContents__action__SavedBindings virtual function #1
   VTable: vtable_SGWTableOfContents__action__SavedBindings at 01965de8 */

void __thiscall SGWTableOfContents__action__SavedBindings__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWTableOfContents__action__actionGroups virtual function #1
   VTable: vtable_SGWTableOfContents__action__actionGroups at 01965dc4 */

void __thiscall SGWTableOfContents__action__actionGroups__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWTableOfContents_action__BindingSaveType virtual function #1
   VTable: vtable_SGWTableOfContents_action__BindingSaveType at 01965da0 */

void __thiscall SGWTableOfContents_action__BindingSaveType__vfunc_1(void *this,undefined4 param_1)

{
  void *this_00;
  void *pvVar1;
  void *pvVar2;
  int local_8 [2];
  
  this_00 = (void *)((int)this + 4);
  *(undefined4 *)((int)this + 0x1c) = param_1;
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
  *(undefined4 *)((int)this + 0x18) = 0;
  return;
}


/* [VTable] SGWTableOfContents_cmd__CommandType virtual function #1
   VTable: vtable_SGWTableOfContents_cmd__CommandType at 01965d7c */

void __thiscall SGWTableOfContents_cmd__CommandType__vfunc_1(void *this,undefined4 param_1)

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


/* [VTable] SGWTableOfContents_toc__ModuleType virtual function #1
   VTable: vtable_SGWTableOfContents_toc__ModuleType at 018fa254 */

void __thiscall SGWTableOfContents_toc__ModuleType__vfunc_1(void *this,undefined4 param_1)

{
  void *pvVar1;
  void *pvVar2;
  void *pvVar3;
  int local_8 [2];
  
  pvVar1 = (void *)((int)this + 0x18);
  *(undefined4 *)((int)this + 0x98) = param_1;
  *(undefined4 *)((int)this + 4) = 0;
  *(undefined4 *)((int)this + 8) = 0;
  *(undefined4 *)((int)this + 0xc) = 0;
  *(undefined4 *)((int)this + 0x10) = 0;
  *(undefined1 *)((int)this + 0x14) = 0;
  pvVar3 = *(void **)((int)this + 0x20);
  if (pvVar3 < *(void **)((int)this + 0x1c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x1c);
  if (*(void **)((int)this + 0x20) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar2,(int)pvVar1,pvVar3);
  pvVar3 = *(void **)((int)this + 0x30);
  pvVar1 = (void *)((int)this + 0x28);
  if (pvVar3 < *(void **)((int)this + 0x2c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x2c);
  if (*(void **)((int)this + 0x30) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar2,(int)pvVar1,pvVar3);
  pvVar3 = *(void **)((int)this + 0x40);
  pvVar1 = (void *)((int)this + 0x38);
  if (pvVar3 < *(void **)((int)this + 0x3c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x3c);
  if (*(void **)((int)this + 0x40) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar2,(int)pvVar1,pvVar3);
  pvVar1 = (void *)((int)this + 0x58);
  *(undefined4 *)((int)this + 0x48) = 0;
  *(undefined4 *)((int)this + 0x4c) = 0;
  *(undefined4 *)((int)this + 0x50) = 0;
  *(undefined4 *)((int)this + 0x54) = 0;
  pvVar3 = *(void **)((int)this + 0x60);
  if (pvVar3 < *(void **)((int)this + 0x5c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x5c);
  if (*(void **)((int)this + 0x60) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar2,(int)pvVar1,pvVar3);
  pvVar3 = *(void **)((int)this + 0x70);
  pvVar1 = (void *)((int)this + 0x68);
  if (pvVar3 < *(void **)((int)this + 0x6c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x6c);
  if (*(void **)((int)this + 0x70) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar2,(int)pvVar1,pvVar3);
  pvVar3 = *(void **)((int)this + 0x80);
  pvVar1 = (void *)((int)this + 0x78);
  if (pvVar3 < *(void **)((int)this + 0x7c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x7c);
  if (*(void **)((int)this + 0x80) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar1,local_8,(int)pvVar1,pvVar2,(int)pvVar1,pvVar3);
  pvVar1 = *(void **)((int)this + 0x90);
  pvVar3 = (void *)((int)this + 0x88);
  if (pvVar1 < *(void **)((int)this + 0x8c)) {
    _invalid_parameter_noinfo();
  }
  pvVar2 = *(void **)((int)this + 0x8c);
  if (*(void **)((int)this + 0x90) < pvVar2) {
    _invalid_parameter_noinfo();
  }
  FUN_0130c770(pvVar3,local_8,(int)pvVar3,pvVar2,(int)pvVar3,pvVar1);
  return;
}


/* [VTable] SGWTableOfContents___cmd__CommandList_sequence virtual function #6
   VTable: vtable_SGWTableOfContents___cmd__CommandList_sequence at 01965ca4 */

void __thiscall
SGWTableOfContents___cmd__CommandList_sequence__vfunc_6
          (void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  SGWTableOfContents__unknown_00b19800(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents__cmd__CommandList virtual function #6
   VTable: vtable_SGWTableOfContents__cmd__CommandList at 01965cc8 */

void __thiscall
SGWTableOfContents__cmd__CommandList__vfunc_6
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  SGWTableOfContents__unknown_00b1a300(param_1,param_2,this,param_3);
  return;
}


/* [VTable] SGWTableOfContents___cmd__CommandList_sequence virtual function #5
   VTable: vtable_SGWTableOfContents___cmd__CommandList_sequence at 01965ca4 */

int __thiscall
SGWTableOfContents___cmd__CommandList_sequence__vfunc_5
          (void *this,char *param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  
  iVar1 = SGWTableOfContents__unknown_00b19800(param_1,param_2,this,param_3);
  if (iVar1 != 0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return iVar1;
}


/* [VTable] SGWTableOfContents__cmd__CommandList virtual function #5
   VTable: vtable_SGWTableOfContents__cmd__CommandList at 01965cc8 */

int * __thiscall
SGWTableOfContents__cmd__CommandList__vfunc_5
          (void *this,char *param_1,byte *param_2,undefined4 param_3)

{
  int *piVar1;
  
  piVar1 = SGWTableOfContents__unknown_00b1a300(param_1,param_2,this,param_3);
  if (piVar1 != (int *)0x0) {
    SGWTableOfContents__unknown_00b1daa0(param_1);
  }
  return piVar1;
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action_defaultBind virtual function #7
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action_defaultBind at 01965cec */

undefined4 * __thiscall
SGWTableOfContents__action__ActionGroupType_action_defaultBind__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::_action__ActionGroupType_action_defaultBind::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0xc,*(int *)((int)this + -4),FUN_00b11b40);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__action__ActionGroupType_action virtual function #7
   VTable: vtable_SGWTableOfContents__action__ActionGroupType_action at 01965d10 */

undefined4 * __thiscall
SGWTableOfContents__action__ActionGroupType_action__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::_action__ActionGroupType_action::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x2c,*(int *)((int)this + -4),FUN_00b11b90);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents___cmd__CommandList_sequence virtual function #7
   VTable: vtable_SGWTableOfContents___cmd__CommandList_sequence at 01965ca4 */

undefined4 * __thiscall
SGWTableOfContents___cmd__CommandList_sequence__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::__cmd__CommandList_sequence::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,8,*(int *)((int)this + -4),FUN_00b11ae0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__toc__ModuleType_Metadata virtual function #7
   VTable: vtable_SGWTableOfContents__toc__ModuleType_Metadata at 01965c38 */

undefined4 * __thiscall
SGWTableOfContents__toc__ModuleType_Metadata__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::_toc__ModuleType_Metadata::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0xc,*(int *)((int)this + -4),FUN_00b11a60);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__toc__ModuleType_LayoutInstance virtual function #7
   VTable: vtable_SGWTableOfContents__toc__ModuleType_LayoutInstance at 01965c14 */

undefined4 * __thiscall
SGWTableOfContents__toc__ModuleType_LayoutInstance__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::_toc__ModuleType_LayoutInstance::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0xc,*(int *)((int)this + -4),FUN_00b11a30);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_action__KeyBindType virtual function #7
   VTable: vtable_SGWTableOfContents_action__KeyBindType at 01965d58 */

undefined4 * __thiscall SGWTableOfContents_action__KeyBindType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::action__KeyBindType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b11bf0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_action__ActionGroupType virtual function #7
   VTable: vtable_SGWTableOfContents_action__ActionGroupType at 01965d34 */

undefined4 * __thiscall SGWTableOfContents_action__ActionGroupType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::action__ActionGroupType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x1c,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b11bc0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__cmd__CommandList virtual function #7
   VTable: vtable_SGWTableOfContents__cmd__CommandList at 01965cc8 */

undefined4 * __thiscall SGWTableOfContents__cmd__CommandList__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::_cmd__CommandList::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x10,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b11b10);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_cmd__OptionalParamType virtual function #7
   VTable: vtable_SGWTableOfContents_cmd__OptionalParamType at 01965c80 */

undefined4 * __thiscall SGWTableOfContents_cmd__OptionalParamType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::cmd__OptionalParamType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x14,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b11ac0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_cmd__MandatoryParamType virtual function #7
   VTable: vtable_SGWTableOfContents_cmd__MandatoryParamType at 01965c5c */

undefined4 * __thiscall SGWTableOfContents_cmd__MandatoryParamType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    *(undefined ***)this = SGWTableOfContents::cmd__MandatoryParamType::vftable;
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_
            (this,0x14,*(int *)((int)this + -4),(_func_void_void_ptr *)&LAB_00b11a90);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__action__SavedBindings virtual function #7
   VTable: vtable_SGWTableOfContents__action__SavedBindings at 01965de8 */

undefined4 * __thiscall SGWTableOfContents__action__SavedBindings__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00b1ed60(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_00b1ed60);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents__action__actionGroups virtual function #7
   VTable: vtable_SGWTableOfContents__action__actionGroups at 01965dc4 */

undefined4 * __thiscall SGWTableOfContents__action__actionGroups__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00b1ecb0(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x18,*(int *)((int)this + -4),FUN_00b1ecb0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_action__BindingSaveType virtual function #7
   VTable: vtable_SGWTableOfContents_action__BindingSaveType at 01965da0 */

undefined4 * __thiscall SGWTableOfContents_action__BindingSaveType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00b1ec00(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x20,*(int *)((int)this + -4),FUN_00b1ec00);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_cmd__CommandType virtual function #7
   VTable: vtable_SGWTableOfContents_cmd__CommandType at 01965d7c */

undefined4 * __thiscall SGWTableOfContents_cmd__CommandType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_00b1eb30(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x3c,*(int *)((int)this + -4),FUN_00b1eb30);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


/* [VTable] SGWTableOfContents_toc__ModuleType virtual function #7
   VTable: vtable_SGWTableOfContents_toc__ModuleType at 018fa254 */

undefined4 * __thiscall SGWTableOfContents_toc__ModuleType__vfunc_7(void *this,byte param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_0093ebb0(this);
    if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
      scalable_free(this);
    }
    return this;
  }
  _eh_vector_destructor_iterator_(this,0x9c,*(int *)((int)this + -4),FUN_0093ebb0);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free((undefined4 *)((int)this + -4));
  }
  return (undefined4 *)((int)this + -4);
}


undefined4 * __thiscall SGWTextCommandMgr__vfunc_0(void *this,byte param_1)

{
  FUN_00c8e1e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGWScriptedWindow__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f87e8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = SGWScriptedWindow::vftable;
  local_4 = 0xffffffff;
  FUN_0094a380(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommonLoaded___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7010(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_WorldChanged___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7460(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_PlayerCreated___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd75d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TargetChanged___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7740(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MouseOverChanged___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7a20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_PetChanged___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7b90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_SquadChanged___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7d00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_SquadChangedAll___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7e70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_SquadInvite___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd7ff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TeamInvite___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd8160(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommandInvite___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd82d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TeamCreate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd8440(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommandCreate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd85b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_UpdateTeamMember___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd8a00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MissionLogUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd8ce0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MissionUpdated___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd8e50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MissionReceived___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd8fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MissionRemoved___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd9130(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MissionRewards___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd9410(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_DialogDisplay___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd9580(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_DialogAvailable___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd96f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_DialogRemoved___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd9860(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_GreetDisplay___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd99d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_InteractionTimer___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd9b40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_InventoryHideSlot___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cd9f90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_InventoryCooldown___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cda100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_InventoryClear___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cda270(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TeamUpdateCash___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cda830(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommandUpdateCash___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cda9a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_VaultVisibility___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdab10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_VendorOpen___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdaf60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_VendorClose___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdb0e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_VendorUpdateSlot___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdb250(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MailUpdateMailbox___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdb6a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MailUpdateMessage___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdb980(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_ExperienceUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdbdd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_DHDVisibility___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdbf40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MinigameName___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdc220(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MinigameCallHide___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdd920(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MinigameObserver___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdda90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_AbilityUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cddc00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_Input_KeyPress___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cddd70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_LoginSuccess___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cddee0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_LoginFailed___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cde050(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_PreRender___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdea60(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_UnitCombat___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdebd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CombatState___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cded40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_AbilityStart___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdeeb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_AbilityEnd___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_PetUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_PetStanceChange___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf300(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_LootDisplay___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf470(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_BMViewReady___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf5e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_BMOpen___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf750(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_BMError___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdf8c0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TrainerUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdfa30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TrainerOpen___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdfba0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_ActionUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdfd10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_ActionCooldown___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdfe80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TradeLocalUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cdfff0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TradeRemoteUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce0160(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_TradeResult___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce02d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommTellSent___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce0720(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommChannelJoined___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce0890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_CommChannelLeft___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce0a00(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_BlueprintUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce0fc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_NetDisconnect___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce1580(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_onDuelChallenge___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce16f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_DuelTimerStart___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce1860(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_SpaceQueued___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce19d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_SpaceQueueReady___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce1b40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_SpaceDestroyed___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce1cb0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MovieStart___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce1e20(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_MovieStop___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce1f90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_StrikeTeamUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce2100(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_ContactListUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce23e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_ContactListDelete___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce2550(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_VEvent_NetIn_onEndAidWait___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce2b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_UnitEffectsUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce8c30(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_VEvent_Entity_StatUpdate___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce8f10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_AbilityCooldown___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce91f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_AutoCycle___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce9360(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_UnitStealth___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce94d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_LevelChanged___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce9640(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_UEvent_UI_EntityReload___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00ce97b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWScriptedWindow_X_VEvent_NetIn_onBeginAidWait___GameEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00cea620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGWAppearanceLog__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01685740;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = SGWAppearanceLog::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this = FOutputDevice::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* Also in vtable: SGWSystemOptions__SGWSystemOptions__OptionGroupType_option (slot 0)
   Also in vtable: SGWTableOfContents__action__SavedBindings (slot 0) */

undefined4 CookedData_CookedData__InteractionSetMapType__vfunc_0(void)

{
  return 0x14;
}


undefined4 * __thiscall SGW_WorldInfo__vfunc_0(void *this,byte param_1)

{
  FUN_00d2e0b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGW_AppliedScience__vfunc_0(void *this,byte param_1)

{
  FUN_00d2e920(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGW_RacialParadigm__vfunc_0(void *this,byte param_1)

{
  FUN_00d2ea40(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGW_Discipline__vfunc_0(void *this,byte param_1)

{
  FUN_00d2eba0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGW_Blueprint__vfunc_0(void *this,byte param_1)

{
  FUN_00d2f290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGW_InteractionData__vfunc_0(void *this,byte param_1)

{
  FUN_00d305f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_onSpaceQueueStatus___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55090(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_contactListCreate___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d551d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_contactListDelete___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_contactListRename___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55450(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_versionInfoRequest___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_elementDataRequest___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55a90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_CreateCharacter___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55bd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DeleteCharacter___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55d10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Disconnect___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55e50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PlayCharacter___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d55f90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_onClientVersion___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56210(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LogOff___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56490(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetCrouched___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d565d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetTargetID___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56710(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetMovementType___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_UseItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56990(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RequestAmmoChange___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56ad0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ListItems___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56c10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_UseAbility___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56d50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LootItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d56fd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ClientReady___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57110(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_AbandonMission___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57250(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShareMission___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57390(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChosenRewards___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57610(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MoveItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57750(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RemoveItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatJoin___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d579d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatLeave___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatSetAFKMessage___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57c50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatSetDNDMessage___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57d90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatIgnore___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d57ed0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatFriend___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58010(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatList___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58150(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatMute___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatKick___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d583d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatOp___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatBan___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58650(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChatPassword___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Petition___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d588d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_OrganizationInvite___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58a10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_OrganizationLeave___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58dd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_OrganizationKick___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d58f10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_OrganizationMOTD___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d59190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_OrganizationNote___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d592d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SquadSetLootMode___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d59910(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Who___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d59a50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RepairItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d59b90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Interact___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d59e10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_InitialResponse___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d59f50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DialogButtonChoice___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a090(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PurchaseItems___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a1d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SellItems___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_BuybackItems___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a450(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RepairItems___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a590(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RechargeItems___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a6d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_TrainAbility___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RequestMailHeaders___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5a950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SendMailMessage___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5aa90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RequestMailBody___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5abd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ArchiveMailMessage___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ad10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DeleteMailMessage___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ae50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ReturnMailMessage___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5af90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetAutoCycle___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5b490(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RequestReload___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5b5d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Respawn___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5b710(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_callForAid___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5b850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SystemOptions___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5b990(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PerfStats___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5bad0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_CancelMovie___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5bc10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_TradeRequest___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5bd50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_TradeRequestCancel___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5be90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_TradeProposal___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5bfd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_TradeLockState___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c110(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Craft___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c250(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Research___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c390(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ReverseEngineer___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c4d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Alloy___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c610(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RespecCraft___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RespecAbility___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5c9d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionAssign___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5cc50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionClear___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5cd90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionClearActive___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ced0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionClearHistory___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d010(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionList___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d150(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionListFull___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionDetails___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d3d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionAdvance___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionReset___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d650(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionComplete___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionSetAvailable___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5d8d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MissionAbandon___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5da10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowTargetLocation___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5db50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowMobCount___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5dc90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowRotation___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ddd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ListAbilities___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5df10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_XRayEyes___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e050(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Invisible___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Physics___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e2d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveStargateAddress___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e410(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowPointSet___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e690(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowFlag___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e7d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ListInteractions___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5e910(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowIP___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ea50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowInventory___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5eb90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowPlayer___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ecd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveXp___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ee10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ef50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveNaqahdah___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f090(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GMRemoveItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f1d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveAbility___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveRespawner___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f450(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveTrainingPoints___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f590(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetGodMode___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetNoXP___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5f950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetNoAggro___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5fa90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetSpeed___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5fbd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetHealth___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5fd10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetHealthMax___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5fe50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetFocus___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d5ff90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetFocusMax___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d600d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetFlag___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60210(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetMobStance___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60350(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetMobAbilitySet___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60490(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetTarget___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d605d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetLevel___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60710(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ResetAbilities___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveAllAbilities___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60990(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Respec___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60ad0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DHD___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60c10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Goto___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60d50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GotoLocation___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60e90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GotoXYZ___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d60fd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ReloadOrganizations___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61110(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ReloadInventory___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61250(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Users___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61390(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetHideGM___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d614d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Summon___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61610(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PrintStats___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61890(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_AbilityDebug___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d619d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_CombatDebug___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61b10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_CombatDebugVerbose___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61c50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_HealDebug___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61d90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_debugStartMinigame___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d61ed0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_debugJoinMinigame___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62150(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DebugAbilityOnMob___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DebugBehaviorsOnMob___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d623d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DebugPathsOnMob___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DebugEvents___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62650(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MobData___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DebugInteract___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d628d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_AddBehaviorEventSet___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62b50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PerfStatsByChannel___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d62dd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_StartMinigame___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63050(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_EndMinigame___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63190(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RequestSpectateList___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63550(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SpectateMinigame___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63690(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MinigameStartCancel___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d637d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MinigameCallRequest___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63910(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MinigameCallAbort___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63a50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MinigameCallAccept___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63b90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MinigameCallDecline___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63cd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_onDialGate___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d63f50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_MinigameComplete___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d641d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveMinigameContact___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64310(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Spawn___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64590(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Despawn___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d646d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_RechargeItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetMobAttribute___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64950(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GetMobAttribute___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64a90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Kill___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64bd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SendGMShout___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64d10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_TestLOS___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64e50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ToggleCombatLOS___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d64f90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ChangeCoverWeight___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65210(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadConstants___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65490(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadAbility___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d655d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadNACSI___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65710(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadAbilitySet___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65850(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadBehavior___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65990(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadMOB___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65ad0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadInteractionSet___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65c10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadItem___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65d50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_LoadMission___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65e90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetMobVariable___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d65fd0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PetInvokeAbility___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66110(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PetAbilityToggle___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66250(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_PetChangeStance___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66390(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_WorldInstanceReset___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d664d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_GiveExpertise___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66610(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_Unstuck___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66c50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DuelChallenge___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66d90(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DuelResponse___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d66ed0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_DuelForfeit___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d67010(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_EnterErrorAIState___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d67150(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ExitErrorAIState___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d67290(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ConfirmEffect___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d673d0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowInstanceFlag___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d67510(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_SetInstanceFlag___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d67650(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall
SGWNetworkManager_VEvent_NetOut_ShowNavigation___EventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00d67790(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGW_Social_ContactList__vfunc_0(void *this,byte param_1)

{
  FUN_00e63740(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 ASGWGamePawn__vfunc_0(void)

{
  return 1;
}


undefined4 ASGWOfflineGamePawn__vfunc_0(void)

{
  return 1;
}


undefined4 ASGWSequenceTargetDummy__vfunc_0(void)

{
  return 1;
}


undefined4 ASGWCamera_Player__vfunc_0(void)

{
  return 0;
}


undefined4 ASGWGameInfo__vfunc_0(void)

{
  return 1;
}


undefined4 ASGWLOSActor__vfunc_0(void)

{
  return 1;
}


undefined4 ASGWController_Player__vfunc_0(void)

{
  return 1;
}


undefined4 * __thiscall SGWMeshCompositor__vfunc_0(void *this,byte param_1)

{
  FUN_00eaaf10(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


undefined4 * __thiscall SGWMaterialCompositor__vfunc_0(void *this,byte param_1)

{
  FUN_00eac720(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: cme_hwprofile_cmebase__NVP (slot 0)
   Also in vtable: SGWTableOfContents_action__KeyBindType (slot 0) */

undefined4 CookedData_CookedData__DialogButtonType__vfunc_0(void)

{
  return 0x11;
}


/* Also in vtable: cme_hwprofile_cmehwprofile__Device (slot 0)
   Also in vtable: SGWTableOfContents_cmd__CommandType (slot 0) */

undefined4 CookedData_CookedData__MissionObjectiveType__vfunc_0(void)

{
  return 0xe;
}


undefined4 * __thiscall SGWMessageQueue__vfunc_0(void *this,byte param_1)

{
  FUN_01562380(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


/* Also in vtable: CMEWinCrashReport_crash__CrashReport (slot 0)
   Also in vtable: SGWUIPersistence_gui__UDim (slot 0)
   Also in vtable: SGWSystemOptions_SGWSystemOptions__OptionDependency (slot 0)
   Also in vtable: SGWBindableActions_action__ActionGroupType (slot 0) */

undefined4 CMEWinCrashReport_crash__CrashReport__vfunc_0(void)

{
  return 7;
}


undefined4
SGWLogin___sgwLogin__SGWLoginResponse_SGWLoginSuccess_SGWShardListResp_sequence__vfunc_0(void)

{
  return 0x3f;
}


undefined4 SGWLogin__sgwLogin__SGWLoginResponse_SGWLoginSuccess_SGWShardListResp__vfunc_0(void)

{
  return 0x3e;
}


undefined4 SGWLogin___ga__GlobalAuthRes_sequence__vfunc_0(void)

{
  return 0x49;
}


undefined4 SGWLogin__sgwLogin__SGWLoginResponse_SGWLoginSuccess_AccountInfo__vfunc_0(void)

{
  return 0x3b;
}


undefined4 SGWLogin__sgwLogin__SGWLoginResponse_SGWLoginSuccess__vfunc_0(void)

{
  return 0x3a;
}


/* Also in vtable: cme_hwprofile_cmehwprofile__Config (slot 0)
   Also in vtable: SGWTableOfContents__cmd__CommandList (slot 0) */

undefined4 CookedData_CookedData__MissionStepsType__vfunc_0(void)

{
  return 0xf;
}




/* === From standalone functions (1 functions) === */

undefined4 * __thiscall quex_SGWGrammarScript__vfunc_0(void *this,byte param_1)

{
  FUN_00ea46b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}



/* === Standalone functions (19) === */

/* [VTable] AStaticMeshActor virtual function #115
   VTable: vtable_AStaticMeshActor at 0184f08c
   Also in vtable: ASGW_TerrainSelStaticMeshActor (slot 115) */

void __fastcall AStaticMeshActor__vfunc_115(int param_1)

{
  *(uint *)(param_1 + 0x74) = *(uint *)(param_1 + 0x74) | 0x40000000;
  return;
}


/* Also in vtable: AStaticMeshActor (slot 84)
   Also in vtable: ASGW_TerrainSelStaticMeshActor (slot 84) */

void __thiscall AActor__vfunc_84(void *this,undefined4 *param_1)

{
  undefined4 *puVar1;
  int *this_00;
  int iStack_60;
  int iStack_5c;
  int iStack_58;
  uint uStack_54;
  int iStack_50;
  int iStack_4c;
  int iStack_48;
  int iStack_44;
  int iStack_40;
  int aiStack_3c [3];
  float fStack_30;
  float fStack_2c;
  float fStack_28;
  float fStack_24;
  float fStack_20;
  float fStack_1c;
  float fStack_18;
  float fStack_14;
  float fStack_10;
  float fStack_c;
  float fStack_8;
  float fStack_4;
  
  if ((*(uint *)((int)this + 0x74) & 0x80000000) == 0) {
    if (DAT_01ee3964 == (undefined4 *)0x0) {
      DAT_01ee3964 = AActor__unknown_00764050();
      AActor__unknown_0075f7c0();
    }
    for (puVar1 = *(undefined4 **)((int)this + 0x34); puVar1 != (undefined4 *)0x0;
        puVar1 = (undefined4 *)puVar1[0xf]) {
      if (puVar1 == DAT_01ee3964) goto LAB_007686fc;
    }
    if (DAT_01ee3964 == (undefined4 *)0x0) {
LAB_007686fc:
      *param_1 = 0;
      param_1[2] = 0;
      if (param_1[1] != 0) {
        param_1[1] = 0x4000;
      }
    }
    this_00 = (int *)((int)this + 0xe8);
    FUN_004f6c10(this_00,aiStack_3c,&uStack_54);
    FUN_004fb310(&uStack_54,&fStack_20);
    FUN_004fb310(param_1,&fStack_30);
    fStack_10 = (fStack_2c * fStack_18 + fStack_24 * fStack_20 + fStack_14 * fStack_30) -
                fStack_28 * fStack_1c;
    fStack_c = (fStack_24 * fStack_1c - fStack_18 * fStack_30) + fStack_14 * fStack_2c +
               fStack_20 * fStack_28;
    fStack_8 = ((fStack_24 * fStack_18 + fStack_30 * fStack_1c) - fStack_2c * fStack_20) +
               fStack_14 * fStack_28;
    fStack_4 = ((fStack_24 * fStack_14 - fStack_20 * fStack_30) - fStack_2c * fStack_1c) -
               fStack_18 * fStack_28;
    FDynamicMeshEmitterData__unknown_004fb280(&iStack_48,&fStack_10);
    iStack_58 = iStack_40 - iStack_4c;
    iStack_60 = iStack_48 - uStack_54;
    iStack_5c = iStack_44 - iStack_50;
    FUN_004f6d00(&iStack_60);
    *this_00 = *this_00 + iStack_60;
    *(int *)((int)this + 0xec) = *(int *)((int)this + 0xec) + iStack_5c;
    *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + iStack_58;
  }
  return;
}


/* [VTable] AStaticMeshActor virtual function #7
   VTable: vtable_AStaticMeshActor at 0184f08c
   Also in vtable: ASGW_TerrainSelStaticMeshActor (slot 7) */

void __fastcall AStaticMeshActor__vfunc_7(int *param_1)

{
  int iVar1;
  
  AActor__vfunc_7(param_1);
  if ((DAT_01ee2684 != 0) && (iVar1 = 0, 0 < param_1[0x6e])) {
    do {
      (**(code **)(*param_1 + 0x11c))(*(undefined4 *)(param_1[0x6d] + iVar1 * 4));
      iVar1 = iVar1 + 1;
    } while (iVar1 < param_1[0x6e]);
  }
  return;
}


/* [VTable] AStaticMeshActor virtual function #83
   VTable: vtable_AStaticMeshActor at 0184f08c
   Also in vtable: ASGW_TerrainSelStaticMeshActor (slot 83) */

void __thiscall AStaticMeshActor__vfunc_83(void *this,float *param_1)

{
  int iVar1;
  int iVar2;
  
  AActor__vfunc_83(this,param_1);
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x1b8)) {
    do {
      iVar1 = *(int *)(*(int *)((int)this + 0x1b4) + iVar2 * 4);
      if ((*(byte *)(iVar1 + 0x268) & 1) != 0) {
        *(float *)(iVar1 + 0x240) = *param_1 + *(float *)(iVar1 + 0x240);
        *(float *)(iVar1 + 0x244) = param_1[1] + *(float *)(iVar1 + 0x244);
        *(float *)(iVar1 + 0x248) = param_1[2] + *(float *)(iVar1 + 0x248);
        (**(code **)(**(int **)(*(int *)((int)this + 0x1b4) + iVar2 * 4) + 0x118))();
      }
      iVar2 = iVar2 + 1;
    } while (iVar2 < *(int *)((int)this + 0x1b8));
  }
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] AStaticMeshActor virtual function #161
   VTable: vtable_AStaticMeshActor at 0184f08c
   Also in vtable: ASGW_TerrainSelStaticMeshActor (slot 161) */

void __fastcall AStaticMeshActor__vfunc_161(wchar_t *param_1)

{
  uint uVar1;
  wchar_t *this;
  int iVar2;
  undefined4 *puVar3;
  undefined **ppuVar4;
  int local_4c [3];
  int local_40 [3];
  int local_34 [3];
  uint local_28 [2];
  void *local_20;
  uint local_18;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016c4fe6;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  AActor__vfunc_161(param_1);
  if (*(int *)(param_1 + 0xd8) == 0) {
    FUN_00498ed0(param_1,local_40);
    local_4 = 0;
    FUN_00df7e30(local_4c);
    local_4 = CONCAT31(local_4._1_3_,1);
    (**(code **)(*DAT_01ea5768 + 0x3c))(1);
    local_18 = local_18 & 0xffffff00;
    FUN_0041b420((int *)&stack0xffffffa0);
    local_18 = 0xffffffff;
    FUN_0041b420((int *)&stack0xffffffac);
    ExceptionList = local_20;
    return;
  }
  if (*(int *)(*(int *)(param_1 + 0xd8) + 0x29c) == 0) {
    FUN_00498ed0(param_1,local_4c);
    local_4 = 2;
    FUN_00df7e30(local_40);
    local_4 = CONCAT31(local_4._1_3_,3);
    (**(code **)(*DAT_01ea5768 + 0x3c))(1);
    local_18 = CONCAT31(local_18._1_3_,2);
    FUN_0041b420((int *)&stack0xffffffac);
    local_18 = 0xffffffff;
    FUN_0041b420((int *)&stack0xffffffa0);
    ExceptionList = local_20;
    return;
  }
  FUN_00555070(local_28);
  uVar1 = local_18;
  while (local_20 == (void *)0x0) {
    local_18 = uVar1;
    if (uVar1 == 0) {
      FUN_00486000("CurrentActor",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\inc\\Engine.h",0x256);
    }
    if ((*(uint *)(uVar1 + 0xc) & 2) != 0) {
      FUN_00486000("!CurrentActor->HasAnyFlags(RF_Unreachable)",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\inc\\Engine.h",599);
    }
    this = (wchar_t *)UEditorEngine__unknown_00607c40(uVar1);
    if ((this != (wchar_t *)0x0) && (this != param_1)) {
      if ((((float)((uint)(*(float *)(this + 0x6e) - *(float *)(param_1 + 0x6e)) & DAT_01816ab0) <
            _DAT_018fde24) &&
          ((((float)((uint)(*(float *)(this + 0x70) - *(float *)(param_1 + 0x70)) & DAT_01816ab0) <
             _DAT_018fde24 &&
            ((float)((uint)(*(float *)(this + 0x72) - *(float *)(param_1 + 0x72)) & DAT_01816ab0) <
             _DAT_018fde24)) && (*(int *)(this + 0xd8) != 0)))) &&
         (((*(int *)(*(int *)(this + 0xd8) + 0x29c) == *(int *)(*(int *)(param_1 + 0xd8) + 0x29c) &&
           (iVar2 = FUN_004fbe80(this + 0x74,(int *)(param_1 + 0x74)), iVar2 != 0)) &&
          (iVar2 = FUN_011748d0(this + 0xa2,(float *)(param_1 + 0xa2)), iVar2 != 0)))) {
        FUN_00498ed0(this,local_34);
        local_4 = 4;
        FUN_00498ed0(param_1,local_4c);
        local_4._0_1_ = 5;
        puVar3 = FUN_00493d60(local_40);
        local_4._0_1_ = 6;
        if (puVar3[1] == 0) {
          ppuVar4 = &PTR_017f8e10;
        }
        else {
          ppuVar4 = (undefined **)*puVar3;
        }
        (**(code **)(*DAT_01ea5768 + 0x3c))(1,param_1,ppuVar4);
        local_4._0_1_ = 5;
        FUN_0041b420(local_40);
        local_4 = CONCAT31(local_4._1_3_,4);
        FUN_0041b420(local_4c);
        local_4 = 0xffffffff;
        FUN_0041b420(local_34);
      }
    }
    FUN_00554730(local_28);
    uVar1 = local_18;
  }
  ExceptionList = local_c;
  return;
}


/* UE3: intASGWCamera_PlayerexecPitch (from registration table) */

void __thiscall ASGWCamera_Player_execPitch(void *this,int param_1)

{
  byte bVar1;
  undefined4 unaff_EDI;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x2fc))(unaff_EDI);
  return;
}


/* UE3: intASGWCamera_PlayerexecYaw (from registration table) */

void __thiscall ASGWCamera_Player_execYaw(void *this,int param_1)

{
  byte bVar1;
  undefined4 unaff_EDI;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x300))(unaff_EDI);
  return;
}


/* UE3: intASGWCamera_PlayerexecYawAbsolute (from registration table) */

void __thiscall ASGWCamera_Player_execYawAbsolute(void *this,int param_1)

{
  byte bVar1;
  undefined4 unaff_EDI;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x304))(unaff_EDI);
  return;
}


/* UE3: intASGWCamera_PlayerexecPitchAbsolute (from registration table) */

void __thiscall ASGWCamera_Player_execPitchAbsolute(void *this,int param_1)

{
  byte bVar1;
  undefined4 unaff_EDI;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x308))(unaff_EDI);
  return;
}


/* UE3: intASGWController_PlayerexecFly (from registration table) */

void __thiscall ASGWController_Player_execFly(void *this,int param_1)

{
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x3d0))();
  return;
}


/* UE3: intASGWController_PlayerexecWalk (from registration table) */

void __thiscall ASGWController_Player_execWalk(void *this,int param_1)

{
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x3d4))();
  return;
}


/* UE3: intASGWController_PlayerexecGhost (from registration table) */

void __thiscall ASGWController_Player_execGhost(void *this,int param_1)

{
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x3d8))();
  return;
}


/* UE3: intASGWController_PlayerexecSloMo (from registration table) */

void __thiscall ASGWController_Player_execSloMo(void *this,int param_1)

{
  byte bVar1;
  undefined4 unaff_EDI;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x3dc))(unaff_EDI);
  return;
}


/* UE3: intASGWController_PlayerexecSetSpeed (from registration table) */

void __thiscall ASGWController_Player_execSetSpeed(void *this,int param_1)

{
  byte bVar1;
  undefined4 unaff_EDI;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  (**(code **)(*(int *)this + 0x3e0))(unaff_EDI);
  return;
}


/* UE3: intUSGWAnim_BlendListexecGetBlendTime (from registration table) */

void __thiscall USGWAnim_BlendList_execGetBlendTime(void *this,int param_1)

{
  byte bVar1;
  float *unaff_ESI;
  int unaff_EDI;
  float10 fVar2;
  int iVar3;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  iVar3 = param_1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  DAT_01ea5798 = DAT_01ea5798 & 0xfffffffd;
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  fVar2 = FUN_00e90910((int)this,iVar3,(uint)(unaff_EDI != 0));
  *unaff_ESI = (float)fVar2;
  return;
}


/* UE3: intUSGWAnim_BlendListexecGetAnimDuration (from registration table) */

void __thiscall USGWAnim_BlendList_execGetAnimDuration(void *this,int param_1)

{
  byte bVar1;
  int unaff_EDI;
  float10 fVar2;
  float *unaff_retaddr;
  undefined4 local_4;
  
  local_4 = 0;
  bVar1 = **(byte **)(param_1 + 0xc);
  *(byte **)(param_1 + 0xc) = *(byte **)(param_1 + 0xc) + 1;
  (*(code *)(&DAT_01edcbd0)[bVar1])(param_1,&local_4);
  *(int *)(param_1 + 0xc) = *(int *)(param_1 + 0xc) + 1;
  if (**(char **)(param_1 + 0xc) == 'A') {
    *(char **)(param_1 + 0xc) = *(char **)(param_1 + 0xc) + 1;
    (*DAT_01edccd4)(param_1,0);
  }
  fVar2 = FUN_00e90a10((int)this,unaff_EDI);
  *unaff_retaddr = (float)fVar2;
  return;
}


/* [String discovery] Debug string: "getTargetCollectionMethod() ==
   SGW::ETargetCollectionMethod::TCM_AERadius"
   String at: 019bb538 */

undefined4 __fastcall SGW_ETargetCollectionMethod_TCM_AERadius(int param_1)

{
  if (*(int *)(param_1 + 0x94) != 2) {
    FUN_00486000("getTargetCollectionMethod() == SGW::ETargetCollectionMethod::TCM_AERadius",
                 ".\\Src\\Ability.cpp",0x60);
  }
  return *(undefined4 *)(param_1 + 0xa0);
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [String discovery] Debug string: "abilityInfo->getTargetType()==SGW::ETargetType::TargetGround"
   String at: 019d2b9c */

void __thiscall SGW_ETargetType_TargetGround(void *this,int param_1)

{
  uint *puVar1;
  float fVar2;
  int iVar3;
  int iVar4;
  void *pvVar5;
  undefined *this_00;
  undefined1 *puVar6;
  float fVar7;
  undefined1 auStack_28 [28];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0170720b;
  local_c = ExceptionList;
  if ((*(char *)this == '\0') && (param_1 != 0)) {
    ExceptionList = &local_c;
    if (*(int *)((int)this + 0x14) != 0) {
      puVar1 = (uint *)(*(int *)((int)this + 0x14) + 0x3c);
      ExceptionList = &local_c;
      *puVar1 = *puVar1 | 2;
      if (*(int *)(param_1 + 0x48) != 3) {
        FUN_00486000("abilityInfo->getTargetType()==SGW::ETargetType::TargetGround",
                     ".\\Src\\GameProxyPlayer.cpp",0x5aa);
      }
      if (*(int *)(param_1 + 0x94) != 2) {
        ExceptionList = local_c;
        return;
      }
      iVar3 = SGW_ETargetCollectionMethod_TCM_AERadius(param_1);
      iVar4 = FUN_00d29e00(param_1);
      fVar7 = (float)iVar4;
      if (iVar4 < 0) {
        fVar7 = fVar7 + _DAT_01810160;
      }
      iVar4 = FUN_00d29e30(param_1);
      fVar2 = (float)iVar4;
      if (iVar4 < 0) {
        fVar2 = fVar2 + _DAT_01810160;
      }
      FUN_00eadf00(*(void **)((int)this + 0x14),(float)iVar3,(float)iVar3,fVar2,fVar7);
      pvVar5 = (void *)__RTDynamicCast(*(undefined4 *)((int)this + 8),0,
                                       &GameEntityBase::RTTI_Type_Descriptor,
                                       &GameBeing::RTTI_Type_Descriptor,0);
      if (pvVar5 != (void *)0x0) {
        FUN_00e00490(pvVar5,*(int *)((int)this + 0x14));
      }
    }
    pvVar5 = (void *)thunk_FUN_0054c900();
    FUN_00dfb410(pvVar5,0,this,FUN_00dea240);
    *(undefined1 *)this = 1;
    FUN_004398f0(auStack_28,L"systemTarget");
    uStack_4 = 0;
    puVar6 = auStack_28;
    this_00 = NVSystemOptionPolicyBool__unknown_0057d070();
    FUN_0057b800(this_00,puVar6);
    uStack_4 = 0xffffffff;
    FUN_00433f40((int)auStack_28);
  }
  ExceptionList = local_c;
  return;
}


undefined4 * __thiscall quex_SGWGrammarScript__vfunc_0(void *this,byte param_1)

{
  FUN_00ea46b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}


