/*
 * SGW.exe Decompilation - 09_game_ui_visuals
 * Classes: 41
 * Stargate Worlds Client
 */


/* ========== BodyVisualizationManager_IPolicyDispatch.c ========== */

/*
 * SGW.exe - BodyVisualizationManager_IPolicyDispatch (1 functions)
 */

undefined4 * __thiscall BodyVisualizationManager_IPolicyDispatch__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = BodyVisualizationManager::IPolicyDispatch::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ComponentAppearanceProxy_AttachHandler.c ========== */

/*
 * SGW.exe - ComponentAppearanceProxy_AttachHandler (1 functions)
 */

undefined4 * __thiscall ComponentAppearanceProxy_AttachHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = ComponentAppearanceProxy::AttachHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CompositeMaterialTask.c ========== */

/*
 * SGW.exe - CompositeMaterialTask (1 functions)
 */

undefined4 * __thiscall CompositeMaterialTask__vfunc_0(void *this,byte param_1)

{
  FUN_00ec3a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CompositeMeshTask.c ========== */

/*
 * SGW.exe - CompositeMeshTask (1 functions)
 */

undefined4 * __thiscall CompositeMeshTask__vfunc_0(void *this,byte param_1)

{
  FUN_00ec3ca0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CompositedAppearanceJob.c ========== */

/*
 * SGW.exe - CompositedAppearanceJob (1 functions)
 */

undefined4 * __thiscall CompositedAppearanceJob__vfunc_0(void *this,byte param_1)

{
  FUN_00eb58e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CompositedAppearanceProxy.c ========== */

/*
 * SGW.exe - CompositedAppearanceProxy (1 functions)
 */

undefined4 * __thiscall CompositedAppearanceProxy__vfunc_0(void *this,byte param_1)

{
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01713258;
  pvStack_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &pvStack_c;
  FUN_00ebf6e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvStack_c;
  return this;
}




/* ========== ExternalWindowModule.c ========== */

/*
 * SGW.exe - ExternalWindowModule (1 functions)
 */

/* Also in vtable: ExternalWindowModule (slot 0) */

undefined4 * __thiscall ExternalWindowModule__vfunc_0(void *this,byte param_1)

{
  FUN_00a28a80(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FBinkMovieRenderClient.c ========== */

/*
 * SGW.exe - FBinkMovieRenderClient (1 functions)
 */

/* Also in vtable: FBinkMovieRenderClient (slot 0) */

undefined4 * __thiscall FBinkMovieRenderClient__vfunc_0(void *this,undefined4 param_1,byte param_2)

{
  int iVar1;
  
  *(undefined ***)this = FBinkMovieRenderClient::vftable;
  if (*(int *)((int)this + 4) != 0) {
    if (*(int *)((int)this + 0x18) != 0) {
      FUN_005f0d20(*(int **)((int)this + 0x14));
      iVar1 = *(int *)((int)this + 0x14);
      if (iVar1 != 0) {
        UCodecMovieBink__unknown_0050c540(iVar1);
                    /* WARNING: Subroutine does not return */
        scalable_free(iVar1);
      }
      *(undefined4 *)((int)this + 0x14) = 0;
      *(undefined4 *)((int)this + 0x18) = 0;
    }
    *(undefined4 *)((int)this + 4) = 0;
  }
  *(undefined4 *)((int)this + 8) = 0;
  if ((param_2 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FBinkTextureSet.c ========== */

/*
 * SGW.exe - FBinkTextureSet (2 functions)
 */

/* [VTable] FBinkTextureSet virtual function #1
   VTable: vtable_FBinkTextureSet at 0181cb24 */

void __fastcall FBinkTextureSet__vfunc_1(int param_1)

{
  int *piVar1;
  int iVar2;
  uint local_c;
  int local_8 [2];
  
  local_8[0] = param_1 + 0x14;
  local_8[1] = param_1 + 0x44;
  local_c = 0;
  do {
    piVar1 = (int *)local_8[local_c];
    piVar1[1] = 0;
    if (piVar1[2] != 0) {
      iVar2 = *piVar1;
      piVar1[2] = 0;
      if (iVar2 != 0) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        iVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,0,8);
        *piVar1 = iVar2;
      }
    }
    piVar1[4] = 0;
    if (piVar1[5] != 0) {
      iVar2 = piVar1[3];
      piVar1[5] = 0;
      if (iVar2 != 0) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        iVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,0,8);
        piVar1[3] = iVar2;
      }
    }
    piVar1[7] = 0;
    if (piVar1[8] != 0) {
      iVar2 = piVar1[6];
      piVar1[8] = 0;
      if (iVar2 != 0) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        iVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,0,8);
        piVar1[6] = iVar2;
      }
    }
    piVar1[10] = 0;
    if (piVar1[0xb] != 0) {
      iVar2 = piVar1[9];
      piVar1[0xb] = 0;
      if (iVar2 != 0) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        iVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,0,8);
        piVar1[9] = iVar2;
      }
    }
    local_c = local_c + 1;
  } while (local_c < 2);
  piVar1 = *(int **)(param_1 + 0xec);
  *(undefined4 *)(param_1 + 0xec) = 0;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  piVar1 = *(int **)(param_1 + 0xf4);
  *(undefined4 *)(param_1 + 0xf4) = 0;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  piVar1 = *(int **)(param_1 + 0xfc);
  *(undefined4 *)(param_1 + 0xfc) = 0;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  piVar1 = *(int **)(param_1 + 0x104);
  *(undefined4 *)(param_1 + 0x104) = 0;
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  *(undefined4 *)(param_1 + 0x10c) = 1;
  return;
}


/* Also in vtable: FBinkTextureSet (slot 0) */

void __fastcall FBinkTextureSet__vfunc_0(int param_1)

{
  int *piVar1;
  int iVar2;
  int *piVar3;
  size_t sVar4;
  uint uVar5;
  undefined4 *puVar6;
  int *local_14;
  int local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0169381b;
  local_c = ExceptionList;
  local_14 = (int *)(param_1 + 0x14);
  local_10 = param_1 + 0x44;
  uVar5 = 0;
  puVar6 = (undefined4 *)(param_1 + 0x94);
  ExceptionList = &local_c;
  do {
    if (*(int *)(param_1 + 0x8c) != 0) {
      sVar4 = *(int *)(param_1 + 0x7c) * *(int *)(param_1 + 0x78);
      iVar2 = FUN_0041a950((&local_14)[uVar5],sVar4,1,8);
      piVar1 = (&local_14)[uVar5];
      memset((void *)(*piVar1 + iVar2),0,sVar4);
      puVar6[-1] = *piVar1;
      *puVar6 = *(undefined4 *)(param_1 + 0x78);
    }
    if (*(int *)(param_1 + 0x98) != 0) {
      sVar4 = *(int *)(param_1 + 0x84) * *(int *)(param_1 + 0x80);
      piVar1 = (&local_14)[uVar5] + 3;
      iVar2 = FUN_0041a950(piVar1,sVar4,1,8);
      memset((void *)(*piVar1 + iVar2),0,sVar4);
      puVar6[2] = *piVar1;
      puVar6[3] = *(undefined4 *)(param_1 + 0x80);
    }
    if (*(int *)(param_1 + 0xa4) != 0) {
      sVar4 = *(int *)(param_1 + 0x84) * *(int *)(param_1 + 0x80);
      piVar1 = (&local_14)[uVar5] + 6;
      iVar2 = FUN_0041a950(piVar1,sVar4,1,8);
      memset((void *)(*piVar1 + iVar2),0,sVar4);
      puVar6[5] = *piVar1;
      puVar6[6] = *(undefined4 *)(param_1 + 0x80);
    }
    if (*(int *)(param_1 + 0xb0) != 0) {
      sVar4 = *(int *)(param_1 + 0x7c) * *(int *)(param_1 + 0x78);
      piVar1 = (&local_14)[uVar5] + 9;
      iVar2 = FUN_0041a950(piVar1,sVar4,1,8);
      memset((void *)(*piVar1 + iVar2),0,sVar4);
      puVar6[8] = *piVar1;
      puVar6[9] = *(undefined4 *)(param_1 + 0x78);
    }
    uVar5 = uVar5 + 1;
    puVar6 = puVar6 + 0xc;
  } while (uVar5 < 2);
  if (*(int *)(param_1 + 0x8c) != 0) {
    piVar3 = FSceneRenderTargets__unknown_00ec8070
                       ((int *)&local_14,*(LPCSTR *)(param_1 + 0x78),*(undefined4 *)(param_1 + 0x7c)
                        ,&DAT_00000003,1,0x10);
    local_4 = 0;
    piVar1 = *(int **)(param_1 + 0xec);
    *(uint *)(param_1 + 0xf0) =
         *(uint *)(param_1 + 0xf0) ^ (piVar3[1] ^ *(uint *)(param_1 + 0xf0)) & 1;
    piVar3 = (int *)*piVar3;
    *(int **)(param_1 + 0xec) = piVar3;
    if (piVar3 != (int *)0x0) {
      (**(code **)(*piVar3 + 4))(piVar3);
    }
    if (piVar1 != (int *)0x0) {
      (**(code **)(*piVar1 + 8))(piVar1);
    }
    local_4 = 0xffffffff;
    if (local_14 != (int *)0x0) {
      (**(code **)(*local_14 + 8))(local_14);
    }
  }
  if (*(int *)(param_1 + 0x98) != 0) {
    piVar3 = FSceneRenderTargets__unknown_00ec8070
                       ((int *)&local_14,*(LPCSTR *)(param_1 + 0x80),*(undefined4 *)(param_1 + 0x84)
                        ,&DAT_00000003,1,0x10);
    local_4 = 2;
    piVar1 = *(int **)(param_1 + 0xf4);
    *(uint *)(param_1 + 0xf8) =
         *(uint *)(param_1 + 0xf8) ^ (piVar3[1] ^ *(uint *)(param_1 + 0xf8)) & 1;
    piVar3 = (int *)*piVar3;
    *(int **)(param_1 + 0xf4) = piVar3;
    if (piVar3 != (int *)0x0) {
      (**(code **)(*piVar3 + 4))(piVar3);
    }
    if (piVar1 != (int *)0x0) {
      (**(code **)(*piVar1 + 8))(piVar1);
    }
    local_4 = 0xffffffff;
    if (local_14 != (int *)0x0) {
      (**(code **)(*local_14 + 8))(local_14);
    }
  }
  if (*(int *)(param_1 + 0xa4) != 0) {
    piVar3 = FSceneRenderTargets__unknown_00ec8070
                       ((int *)&local_14,*(LPCSTR *)(param_1 + 0x80),*(undefined4 *)(param_1 + 0x84)
                        ,&DAT_00000003,1,0x10);
    local_4 = 4;
    piVar1 = *(int **)(param_1 + 0xfc);
    *(uint *)(param_1 + 0x100) =
         *(uint *)(param_1 + 0x100) ^ (piVar3[1] ^ *(uint *)(param_1 + 0x100)) & 1;
    piVar3 = (int *)*piVar3;
    *(int **)(param_1 + 0xfc) = piVar3;
    if (piVar3 != (int *)0x0) {
      (**(code **)(*piVar3 + 4))(piVar3);
    }
    if (piVar1 != (int *)0x0) {
      (**(code **)(*piVar1 + 8))(piVar1);
    }
    local_4 = 0xffffffff;
    if (local_14 != (int *)0x0) {
      (**(code **)(*local_14 + 8))(local_14);
    }
  }
  if (*(int *)(param_1 + 0xb0) != 0) {
    piVar3 = FSceneRenderTargets__unknown_00ec8070
                       ((int *)&local_14,*(LPCSTR *)(param_1 + 0x78),*(undefined4 *)(param_1 + 0x7c)
                        ,&DAT_00000003,1,0x10);
    local_4 = 6;
    piVar1 = *(int **)(param_1 + 0x104);
    *(uint *)(param_1 + 0x108) =
         *(uint *)(param_1 + 0x108) ^ (piVar3[1] ^ *(uint *)(param_1 + 0x108)) & 1;
    piVar3 = (int *)*piVar3;
    *(int **)(param_1 + 0x104) = piVar3;
    if (piVar3 != (int *)0x0) {
      (**(code **)(*piVar3 + 4))(piVar3);
    }
    if (piVar1 != (int *)0x0) {
      (**(code **)(*piVar1 + 8))(piVar1);
    }
    local_4 = 0xffffffff;
    if (local_14 != (int *)0x0) {
      (**(code **)(*local_14 + 8))(local_14);
    }
  }
  ExceptionList = local_c;
  return;
}




/* ========== FBinkVertexDeclaration.c ========== */

/*
 * SGW.exe - FBinkVertexDeclaration (1 functions)
 */

/* [VTable] FBinkVertexDeclaration virtual function #7
   VTable: vtable_FBinkVertexDeclaration at 0184c300 */

undefined4 * __thiscall FBinkVertexDeclaration__vfunc_7(void *this,byte param_1)

{
  int *piVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0179280b;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = FBinkVertexDeclaration::vftable;
  local_4 = 0xffffffff;
  piVar1 = *(int **)((int)this + 0x14);
  if (piVar1 != (int *)0x0) {
    (**(code **)(*piVar1 + 8))(piVar1);
  }
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== FBinkYCrCbAToRGBAPixelShader.c ========== */

/*
 * SGW.exe - FBinkYCrCbAToRGBAPixelShader (1 functions)
 */

/* [VTable] FBinkYCrCbAToRGBAPixelShader virtual function #8
   VTable: vtable_FBinkYCrCbAToRGBAPixelShader at 0184c2a0 */

void __thiscall FBinkYCrCbAToRGBAPixelShader__vfunc_8(void *this,int *param_1)

{
  FUN_005401e0(param_1,(int)this + 0x74);
  FShader__vfunc_8(this,param_1);
  return;
}




/* ========== FBinkYCrCbToRGBNoPixelAlphaPixelShader.c ========== */

/*
 * SGW.exe - FBinkYCrCbToRGBNoPixelAlphaPixelShader (1 functions)
 */

/* [VTable] FBinkYCrCbToRGBNoPixelAlphaPixelShader virtual function #8
   VTable: vtable_FBinkYCrCbToRGBNoPixelAlphaPixelShader at 0184c260 */

void __thiscall FBinkYCrCbToRGBNoPixelAlphaPixelShader__vfunc_8(void *this,int *param_1)

{
  void *pvVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  int iVar5;
  int iVar6;
  int iVar7;
  
  iVar7 = (int)this + 0x70;
  iVar6 = (int)this + 0x6c;
  iVar5 = (int)this + 0x68;
  iVar4 = (int)this + 100;
  iVar3 = (int)this + 0x60;
  iVar2 = (int)this + 0x5c;
  pvVar1 = FUN_005401e0(param_1,(int)this + 0x58);
  pvVar1 = FUN_005401e0(pvVar1,iVar2);
  pvVar1 = FUN_005401e0(pvVar1,iVar3);
  pvVar1 = FUN_005401e0(pvVar1,iVar4);
  pvVar1 = FUN_005401e0(pvVar1,iVar5);
  pvVar1 = FUN_005401e0(pvVar1,iVar6);
  FUN_005401e0(pvVar1,iVar7);
  FShader__vfunc_8(this,param_1);
  return;
}




/* ========== FFullScreenMovieBink.c ========== */

/*
 * SGW.exe - FFullScreenMovieBink (15 functions)
 */

/* [VTable] FFullScreenMovieBink virtual function #2
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

undefined4 __fastcall FFullScreenMovieBink__vfunc_2(int param_1)

{
  if ((*(int *)(param_1 + 0x4c) != 0) && (*(int *)(param_1 + 0xa0) != 0)) {
    return 1;
  }
  return 0;
}


/* [VTable] FFullScreenMovieBink virtual function #14
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

undefined4 FFullScreenMovieBink__vfunc_14(void)

{
  return 1;
}


/* [VTable] FFullScreenMovieBink virtual function #12
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

undefined4 __fastcall FFullScreenMovieBink__vfunc_12(int param_1)

{
  return *(undefined4 *)(param_1 + 0x3c);
}


/* [VTable] FFullScreenMovieBink virtual function #15
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void FFullScreenMovieBink__vfunc_15(void)

{
  FUN_005e7480(1);
  return;
}


/* [VTable] FFullScreenMovieBink virtual function #16
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __thiscall FFullScreenMovieBink__vfunc_16(void *this,undefined4 param_1)

{
  *(undefined4 *)((int)this + 0x60) = 1;
  *(undefined4 *)((int)this + 100) = param_1;
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] FFullScreenMovieBink virtual function #7
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __fastcall FFullScreenMovieBink__vfunc_7(int param_1)

{
  float10 fVar1;
  int iVar2;
  float10 fVar3;
  float10 fVar4;
  LARGE_INTEGER LStack_2c;
  LARGE_INTEGER LStack_24;
  void *pvStack_18;
  void *pvStack_14;
  undefined1 *puStack_10;
  undefined4 uStack_c;
  
  uStack_c = 0xffffffff;
  puStack_10 = &LAB_01688f54;
  pvStack_14 = ExceptionList;
  ExceptionList = &pvStack_14;
  iVar2 = (**(code **)(**(int **)(param_1 + 8) + 0x14))(1);
  while ((iVar2 == 0 && (DAT_01ead7ec == 0))) {
    if (DAT_01ea3f80 == 0) {
      FUN_00491830();
    }
    if ((DAT_01ee1248 & 1) == 0) {
      DAT_01ee1248 = DAT_01ee1248 | 1;
      puStack_10 = (undefined1 *)0x0;
      QueryPerformanceCounter(&LStack_2c);
      _DAT_01ee1240 = (double)(longlong)LStack_2c * DAT_01ead7a0;
      puStack_10 = (undefined1 *)0xffffffff;
    }
    QueryPerformanceCounter(&LStack_24);
    fVar3 = (float10)(longlong)LStack_24;
    fVar1 = (float10)DAT_01ead7a0;
    fVar4 = (float10)_DAT_01ee1240;
    _DAT_01ee1240 = (double)(fVar3 * fVar1);
    if (*(int *)(DAT_01ee1254 + 0x2cc) == 0) {
      FUN_00486000("GEngine->Client",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/FullScreenMovieBink.inl"
                   ,0x218);
    }
    (**(code **)(**(int **)(DAT_01ee1254 + 0x2cc) + 0x110))((float)(fVar3 * fVar1 - fVar4));
    iVar2 = (**(code **)(**(int **)(param_1 + 8) + 0x14))(1);
  }
  ExceptionList = pvStack_18;
  return;
}


/* [VTable] FFullScreenMovieBink virtual function #11
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __fastcall FFullScreenMovieBink__vfunc_11(int *param_1)

{
  wchar_t *pwVar1;
  int iVar2;
  wchar_t *pwVar3;
  
  pwVar3 = L"nostartupmovies";
  pwVar1 = (wchar_t *)FUN_00483290();
  iVar2 = FUN_00484390(pwVar1,pwVar3);
  if ((iVar2 == 0) && (0 < param_1[0x1b])) {
    if (((undefined4 *)param_1[0x1a])[1] != 0) {
      (**(code **)(*param_1 + 0x14))(1,*(undefined4 *)param_1[0x1a],0);
      return;
    }
    (**(code **)(*param_1 + 0x14))(1,&PTR_017f8e10,0);
  }
  return;
}


/* [VTable] FFullScreenMovieBink virtual function #8
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

undefined4 __thiscall FFullScreenMovieBink__vfunc_8(void *this,wchar_t *param_1)

{
  bool bVar1;
  bool bVar2;
  bool bVar3;
  int iVar4;
  void *this_00;
  undefined4 *puVar5;
  undefined **ppuVar6;
  undefined4 uVar7;
  undefined **_Str1;
  int local_24 [3];
  int local_18 [2];
  void *pvStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01688fbc;
  local_c = ExceptionList;
  bVar2 = false;
  bVar1 = false;
  ExceptionList = &local_c;
  iVar4 = _wcsicmp(param_1,L"");
  if (iVar4 != 0) {
    if (*(int *)((int)this + 0x20) == 0) {
      ppuVar6 = &PTR_017f8e10;
    }
    else {
      ppuVar6 = *(undefined ***)((int)this + 0x1c);
    }
    iVar4 = _wcsicmp((wchar_t *)ppuVar6,L"");
    if (iVar4 != 0) {
      this_00 = FUN_0048eff0(local_18,param_1);
      local_4 = 0;
      puVar5 = FUN_00488b90(this_00,local_24,1);
      local_4 = 1;
      bVar2 = true;
      bVar1 = true;
      if (puVar5[1] == 0) {
        ppuVar6 = &PTR_017f8e10;
      }
      else {
        ppuVar6 = (undefined **)*puVar5;
      }
      if (*(int *)((int)this + 0x20) == 0) {
        _Str1 = &PTR_017f8e10;
      }
      else {
        _Str1 = *(undefined ***)((int)this + 0x1c);
      }
      iVar4 = _wcsicmp((wchar_t *)_Str1,(wchar_t *)ppuVar6);
      bVar3 = false;
      if (iVar4 != 0) goto LAB_005095ea;
    }
  }
  bVar3 = true;
LAB_005095ea:
  local_4 = 0;
  if (bVar1) {
    FUN_0041b420(local_24);
  }
  local_4 = 0xffffffff;
  if (bVar2) {
    local_4 = 0xffffffff;
    FUN_0041b420(local_18);
  }
  if (bVar3) {
    uVar7 = (**(code **)(**(int **)((int)this + 8) + 0x14))(0);
    ExceptionList = pvStack_10;
    return uVar7;
  }
  ExceptionList = local_c;
  return 0;
}


/* [VTable] FFullScreenMovieBink virtual function #9
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

bool __thiscall FFullScreenMovieBink__vfunc_9(void *this,wchar_t *param_1)

{
  bool bVar1;
  bool bVar2;
  bool bVar3;
  int iVar4;
  void *this_00;
  undefined4 *puVar5;
  undefined **ppuVar6;
  undefined **_Str1;
  int local_24 [3];
  int local_18 [2];
  void *pvStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01689000;
  local_c = ExceptionList;
  bVar2 = false;
  bVar1 = false;
  ExceptionList = &local_c;
  iVar4 = _wcsicmp(param_1,L"");
  if (iVar4 != 0) {
    if (*(int *)((int)this + 0x20) == 0) {
      ppuVar6 = &PTR_017f8e10;
    }
    else {
      ppuVar6 = *(undefined ***)((int)this + 0x1c);
    }
    iVar4 = _wcsicmp((wchar_t *)ppuVar6,L"");
    if (iVar4 != 0) {
      this_00 = FUN_0048eff0(local_18,param_1);
      local_4 = 0;
      puVar5 = FUN_00488b90(this_00,local_24,1);
      local_4 = 1;
      bVar2 = true;
      bVar1 = true;
      if (puVar5[1] == 0) {
        ppuVar6 = &PTR_017f8e10;
      }
      else {
        ppuVar6 = (undefined **)*puVar5;
      }
      if (*(int *)((int)this + 0x20) == 0) {
        _Str1 = &PTR_017f8e10;
      }
      else {
        _Str1 = *(undefined ***)((int)this + 0x1c);
      }
      iVar4 = _wcsicmp((wchar_t *)_Str1,(wchar_t *)ppuVar6);
      bVar3 = false;
      if (iVar4 != 0) goto LAB_0050973a;
    }
  }
  bVar3 = true;
LAB_0050973a:
  local_4 = 0;
  if (bVar1) {
    FUN_0041b420(local_24);
  }
  local_4 = 0xffffffff;
  if (bVar2) {
    local_4 = 0xffffffff;
    FUN_0041b420(local_18);
  }
  if (bVar3) {
    iVar4 = (**(code **)(**(int **)((int)this + 8) + 0x14))(0);
    ExceptionList = pvStack_10;
    return (bool)('\x01' - (iVar4 != 0));
  }
  ExceptionList = local_c;
  return false;
}


/* [VTable] FFullScreenMovieBink virtual function #10
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void * __thiscall FFullScreenMovieBink__vfunc_10(void *this,void *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01689023;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0041aa40(param_1,(undefined4 *)((int)this + 0x1c));
  ExceptionList = local_c;
  return param_1;
}


/* [VTable] FFullScreenMovieBink virtual function #17
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __fastcall FFullScreenMovieBink__vfunc_17(int param_1)

{
  int iVar1;
  int local_2c;
  int *local_28;
  undefined **local_24;
  void *local_20;
  int local_1c [4];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016893b5;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/FullScreenMovieBink.inl"
                 ,0x5e5);
  }
  if (DAT_01ee2618 != 0) {
    local_28 = FUN_00419650(local_1c,&DAT_01ee2634,8);
    local_24 = (undefined **)local_28[2];
    local_4 = 1;
    if (local_24 != (void *)0x0) {
      local_2c = param_1 + 0xac;
      FUN_005087b0(local_24,&local_2c);
    }
    local_4 = 0xffffffff;
    FUN_00419230(local_1c);
    ExceptionList = local_c;
    return;
  }
  local_20 = (void *)(param_1 + 0xac);
  local_24 = `public:_virtual_void___thiscall_FFullScreenMovieBink::GameThreadRemoveAllOverlays(void)'
             ::__l2::RemoveAllOverlaysCommand::vftable;
  local_4 = 3;
  FUN_0050cdf0(local_20,0);
  ExceptionList = local_c;
  return;
}


/* [VTable] FFullScreenMovieBink virtual function #1
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __thiscall FFullScreenMovieBink__vfunc_1(void *this,float param_1)

{
  float fVar1;
  LARGE_INTEGER local_8;
  
  if ((*(float *)((int)this + 0x38) != 0.0) &&
     (fVar1 = *(float *)((int)this + 0x38) - param_1, *(float *)((int)this + 0x38) = fVar1,
     fVar1 <= 0.0)) {
    if ((*(int *)((int)this + 0x78) == 0) &&
       ((*(int *)((int)this + 0x74) != -1 &&
        (*(int *)((int)this + 0x74) < *(int *)((int)this + 0x6c) + -1)))) {
      *(undefined4 *)((int)this + 0x78) = 1;
    }
    else {
      FUN_0050ad50(this,1);
    }
  }
  QueryPerformanceCounter(&local_8);
  if (*(int *)((int)this + 0xa0) == 0) {
    FUN_00486000("BinkRender",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/FullScreenMovieBink.inl"
                 ,0xbc);
  }
  if (*(int *)((int)this + 0x4c) != 0) {
    FUN_0050b710(this);
  }
  QueryPerformanceCounter(&local_8);
  return;
}


/* [VTable] FFullScreenMovieBink virtual function #5
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __thiscall FFullScreenMovieBink__vfunc_5(void *this,short *param_1,int *param_2)

{
  int iVar1;
  int *piVar2;
  undefined4 uVar3;
  uint unaff_retaddr;
  void *pvStack_60;
  int *piStack_5c;
  void *pvStack_58;
  int iStack_54;
  undefined4 uStack_50;
  int iStack_4c;
  exception aeStack_48 [4];
  undefined **ppuStack_44;
  int iStack_40;
  uint uStack_38;
  int iStack_34;
  undefined4 uStack_30;
  void *pvStack_2c;
  int aiStack_28 [4];
  int *piStack_18;
  undefined4 uStack_14;
  void *pvStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0168955b;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (DAT_01eb0830 != 0) {
    ExceptionList = &pvStack_c;
    FUN_00ed7160();
  }
  iVar1 = (**(code **)(**(int **)((int)this + 8) + 0x14))(0);
  if (iVar1 != 0) {
    FUN_0041aab0(&iStack_54,param_1);
    puStack_8 = (undefined1 *)0x1;
    piVar2 = FUN_00488b90(&iStack_54,aeStack_48,1);
    puStack_8._0_1_ = 2;
    FUN_0041a630((undefined4 *)((int)this + 0x1c),piVar2);
    puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,1);
    FUN_0041b420((int *)aeStack_48);
    puStack_8 = (undefined1 *)0xffffffff;
    FUN_0041b420(&iStack_54);
    if (*(int *)((int)this + 0xa0) == 0) {
      pvStack_60 = (void *)scalable_malloc(0x1c);
      if (pvStack_60 == (void *)0x0) {
        FUN_004099f0(aeStack_48);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(aeStack_48,(ThrowInfo *)&DAT_01d65cc8);
      }
      puStack_8 = (undefined1 *)0x4;
      FUN_00508930(pvStack_60,(int)this + 4);
      puStack_8 = (undefined1 *)0xffffffff;
      iVar1 = FUN_0054bb60();
      if (iVar1 == 0) {
        FUN_00486000("IsInGameThread()",
                     "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/FullScreenMovieBink.inl"
                     ,0x177);
      }
      if (DAT_01ee2618 == 0) {
        pvStack_60 = this;
        FUN_00508550(&iStack_54,(undefined4 *)&stack0xffffff9c,&pvStack_60);
        *(undefined4 *)(iStack_4c + 0xa0) = uStack_50;
        puStack_8 = (undefined1 *)0xffffffff;
        FUN_008e4560(&iStack_54);
      }
      else {
        piStack_5c = FUN_00419650(aeStack_48,&DAT_01ee2634,0xc);
        pvStack_58 = (void *)piStack_5c[2];
        puStack_8 = (undefined1 *)0x6;
        if (pvStack_58 != (void *)0x0) {
          pvStack_60 = this;
          FUN_00508550(pvStack_58,(undefined4 *)&stack0xffffff9c,&pvStack_60);
        }
        puStack_8 = (undefined1 *)0xffffffff;
        FUN_00419230((int *)aeStack_48);
      }
    }
    if (DAT_01ee2618 == 0) {
      *(undefined4 *)((int)this + 0x5c) = DAT_01ee261c;
      DAT_01ee261c = 1;
      FUN_0054bd70();
      *(undefined4 *)((int)this + 0x58) = 1;
    }
    else {
      FUN_0054c170();
    }
    FUN_005e7480(0);
    (**(code **)(**(int **)((int)this + 8) + 0xc))();
    if ((unaff_retaddr & 1) != 0) {
      (**(code **)(**(int **)((int)this + 0xc) + 0xc))();
    }
    if ((*(float **)((int)this + 0xa4) != (float *)0x0) && (*(int *)((int)this + 100) == 0)) {
      FUN_00508810(*(float **)((int)this + 0xa4));
    }
    iVar1 = FDecompResultInterface__unknown_0048e5b0
                      ((void *)((int)this + 0x94),(undefined4 *)((int)this + 0x1c));
    if ((iVar1 == -1) ||
       ((DAT_01ee2684 != 0 &&
        (iVar1 = FUN_0054d270(DAT_01ee2684), *(char *)(iVar1 + 0x310) == '\x03')))) {
      uVar3 = 0;
    }
    else {
      uVar3 = 1;
    }
    FUN_0041aab0(aiStack_28,param_1);
    piStack_18 = param_2;
    puStack_8 = (undefined1 *)0x9;
    uStack_14 = uVar3;
    iVar1 = FUN_0054bb60();
    if (iVar1 == 0) {
      FUN_00486000("IsInGameThread()",
                   "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/FullScreenMovieBink.inl"
                   ,0x1c2);
    }
    if (DAT_01ee2618 == 0) {
      param_1 = this;
      FUN_00509ca0(aeStack_48,aiStack_28,&param_1);
      puStack_8._0_1_ = 0xc;
      if (iStack_40 == 0) {
        ppuStack_44 = &PTR_017f8e10;
      }
      FUN_0050b000(pvStack_2c,uStack_38,(short *)ppuStack_44,0,0,iStack_34,uStack_30);
      puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,9);
      FUN_0050c830((undefined4 *)aeStack_48);
    }
    else {
      param_2 = FUN_00419650(aeStack_48,&DAT_01ee2634,0x20);
      puStack_8._1_3_ = (undefined3)((uint)puStack_8 >> 8);
      pvStack_58 = (void *)param_2[2];
      puStack_8._0_1_ = 0xb;
      if (pvStack_58 != (void *)0x0) {
        param_1 = this;
        FUN_00509ca0(pvStack_58,aiStack_28,&param_1);
      }
      puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,9);
      FUN_00419230((int *)aeStack_48);
    }
    if (((unaff_retaddr & 1) != 0) && (DAT_01ead7ec == 0)) {
      (**(code **)(**(int **)((int)this + 0xc) + 0x14))(0xffffffff);
    }
    puStack_8 = (undefined1 *)0xffffffff;
    FUN_0041b420(aiStack_28);
    ExceptionList = pvStack_10;
    return;
  }
  ExceptionList = pvStack_10;
  return;
}


/* [VTable] FFullScreenMovieBink virtual function #6
   VTable: vtable_FFullScreenMovieBink at 0181cc3c */

void __thiscall FFullScreenMovieBink__vfunc_6(void *this,undefined4 param_1,int param_2,int param_3)

{
  int iVar1;
  void *local_28;
  int *local_24;
  void *local_20;
  undefined **local_1c;
  int local_18;
  void *local_14;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01689596;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/FullScreenMovieBink.inl"
                 ,0x1de);
  }
  if (DAT_01ee2618 == 0) {
    local_1c = `public:_virtual_void___thiscall_FFullScreenMovieBink::GameThreadStopMovie(float,int,int)'
               ::__l2::StopGearsMovieCommand::vftable;
    local_18 = param_3;
    local_4 = 3;
    local_14 = this;
    if ((((param_3 == 0) && (*(int *)((int)this + 0x78) == 0)) && (*(int *)((int)this + 0x74) != -1)
        ) && (*(int *)((int)this + 0x74) < *(int *)((int)this + 0x6c) + -1)) {
      *(undefined4 *)((int)this + 0x78) = 1;
    }
    else {
      FUN_0050ad50(this,(uint)(param_3 == 0));
    }
    local_4 = 0xffffffff;
    local_1c = FRenderCommand::vftable;
  }
  else {
    local_24 = FUN_00419650(&local_1c,&DAT_01ee2634,0xc);
    local_20 = (void *)local_24[2];
    local_4 = 1;
    if (local_20 != (void *)0x0) {
      local_28 = this;
      FUN_005085b0(local_20,&param_3,&local_28);
    }
    local_4 = 0xffffffff;
    FUN_00419230((int *)&local_1c);
  }
  if (param_2 != 0) {
    (**(code **)(*(int *)this + 0x1c))();
  }
  if (*(int *)((int)this + 0x58) == 0) {
    FUN_0054c170();
  }
  else {
    FUN_0054c1c0();
    DAT_01ee261c = *(undefined4 *)((int)this + 0x5c);
    *(undefined4 *)((int)this + 0x58) = 0;
  }
  if (*(undefined4 **)((int)this + 0xa0) != (undefined4 *)0x0) {
    (**(code **)**(undefined4 **)((int)this + 0xa0))(1);
  }
  *(undefined4 *)((int)this + 0xa0) = 0;
  ExceptionList = local_c;
  return;
}


/* Also in vtable: FFullScreenMovieBink (slot 0) */

undefined4 * __thiscall FFullScreenMovieBink__vfunc_0(void *this,byte param_1)

{
  FUN_0050abc0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FFullScreenMovieFallback.c ========== */

/*
 * SGW.exe - FFullScreenMovieFallback (12 functions)
 */

/* [VTable] FFullScreenMovieFallback virtual function #1
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void FFullScreenMovieFallback__vfunc_1(void)

{
  return;
}


/* [VTable] FFullScreenMovieFallback virtual function #2
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

undefined4 FFullScreenMovieFallback__vfunc_2(void)

{
  return 0;
}


/* [VTable] FFullScreenMovieFallback virtual function #5
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void FFullScreenMovieFallback__vfunc_5(void)

{
  return;
}


/* [VTable] FFullScreenMovieFallback virtual function #6
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void FFullScreenMovieFallback__vfunc_6(void)

{
  return;
}


/* [VTable] FFullScreenMovieFallback virtual function #7
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void FFullScreenMovieFallback__vfunc_7(void)

{
  return;
}


/* [VTable] FFullScreenMovieFallback virtual function #8
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

undefined4 FFullScreenMovieFallback__vfunc_8(void)

{
  return 1;
}


/* [VTable] FFullScreenMovieFallback virtual function #9
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

undefined4 FFullScreenMovieFallback__vfunc_9(void)

{
  return 0;
}


/* [VTable] FFullScreenMovieFallback virtual function #11
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void FFullScreenMovieFallback__vfunc_11(void)

{
  return;
}


/* [VTable] FFullScreenMovieFallback virtual function #12
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

undefined4 FFullScreenMovieFallback__vfunc_12(void)

{
  return 0;
}


/* [VTable] FFullScreenMovieFallback virtual function #13
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void FFullScreenMovieFallback__vfunc_13(void)

{
  return;
}


/* [VTable] FFullScreenMovieFallback virtual function #10
   VTable: vtable_FFullScreenMovieFallback at 0181cbb0 */

void * FFullScreenMovieFallback__vfunc_10(void *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016890be;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0041aab0(param_1,(short *)&DAT_0181c800);
  ExceptionList = local_c;
  return param_1;
}


/* Also in vtable: FFullScreenMovieFallback (slot 0) */

undefined4 * __thiscall FFullScreenMovieFallback__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016897c8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_0050c7a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== FFullScreenMovieSupport.c ========== */

/*
 * SGW.exe - FFullScreenMovieSupport (1 functions)
 */

/* Also in vtable: FFullScreenMovieSupport (slot 0) */

undefined4 * __thiscall FFullScreenMovieSupport__vfunc_0(void *this,byte param_1)

{
  FUN_0050c7a0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FlashExternalWindowModule.c ========== */

/*
 * SGW.exe - FlashExternalWindowModule (8 functions)
 */

/* [VTable] FlashExternalWindowModule virtual function #5
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void FlashExternalWindowModule__vfunc_5(void)

{
  return;
}


/* [VTable] FlashExternalWindowModule virtual function #2
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void __thiscall FlashExternalWindowModule__vfunc_2(void *this,int param_1)

{
  undefined *puVar1;
  undefined4 local_14;
  undefined4 local_10;
  undefined4 local_c;
  undefined4 local_8;
  undefined4 local_4;
  
  puVar1 = FlashExternalWindowModule__unknown_00569230();
  if ((*(int *)(puVar1 + 0x48) != 0) && (*(int *)((int)this + 0x24) != 0)) {
    local_4 = 0;
    if (*(int *)(param_1 + 0x1c) == 0) {
      local_4 = 0;
    }
    else if (*(int *)(param_1 + 0x1c) == 1) {
      local_4 = 1;
    }
    local_c = *(undefined4 *)((int)this + 0x2c);
    local_10 = *(undefined4 *)((int)this + 0x28);
    local_14 = 2;
    local_8 = 0;
    (**(code **)(**(int **)(*(int *)((int)this + 0x24) + 0x34) + 0x84))(&local_14);
  }
  return;
}


/* [VTable] FlashExternalWindowModule virtual function #3
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void __fastcall FlashExternalWindowModule__vfunc_3(int param_1)

{
  undefined *puVar1;
  undefined4 local_34;
  undefined4 local_30;
  undefined4 local_2c;
  undefined4 local_28;
  undefined4 local_24;
  undefined4 local_20;
  undefined4 local_1c;
  undefined4 local_18;
  undefined4 local_14;
  undefined4 local_10;
  undefined4 local_c;
  undefined4 local_8;
  undefined4 local_4;
  
  puVar1 = FlashExternalWindowModule__unknown_00569230();
  if ((*(int *)(puVar1 + 0x48) != 0) && (*(int *)(param_1 + 0x24) != 0)) {
    local_34 = 0;
    local_30 = 0;
    local_28 = 0;
    local_2c = 0;
    local_20 = 1;
    local_24 = 1;
    local_10 = 0;
    local_14 = 0;
    local_18 = 0;
    local_1c = 0;
    local_4 = 0;
    local_8 = DAT_017f7ea0;
    local_c = DAT_017f7ea0;
    (**(code **)(**(int **)(*(int *)(param_1 + 0x24) + 0x34) + 0x54))(&local_34);
    (**(code **)(**(int **)(*(int *)(param_1 + 0x24) + 0x34) + 0x84))(&stack0xffffffb4);
  }
  return;
}


/* [VTable] FlashExternalWindowModule virtual function #4
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void __thiscall FlashExternalWindowModule__vfunc_4(void *this,float param_1)

{
  undefined *puVar1;
  float unaff_retaddr;
  undefined4 local_34;
  undefined4 local_30;
  undefined4 local_2c;
  int local_28;
  int local_24;
  undefined4 local_20;
  undefined4 local_1c;
  undefined4 local_18;
  undefined4 local_14;
  undefined4 local_10;
  undefined4 local_c;
  undefined4 local_8;
  undefined4 local_4;
  
  puVar1 = FlashExternalWindowModule__unknown_00569230();
  if ((*(int *)(puVar1 + 0x48) != 0) && (*(int *)((int)this + 0x24) != 0)) {
    local_34 = 0;
    local_30 = 0;
    local_28 = 0;
    local_2c = 0;
    local_20 = 1;
    local_24 = 1;
    local_10 = 0;
    local_14 = 0;
    local_18 = 0;
    local_1c = 0;
    local_4 = 0;
    local_8 = DAT_017f7ea0;
    local_c = DAT_017f7ea0;
    (**(code **)(**(int **)(*(int *)((int)this + 0x24) + 0x34) + 0x54))(&local_34);
    *(int *)((int)this + 0x2c) = (int)((float)local_24 * param_1);
    *(int *)((int)this + 0x28) = (int)((float)local_28 * unaff_retaddr);
    (**(code **)(**(int **)(*(int *)((int)this + 0x24) + 0x34) + 0x84))(&stack0xffffffb4);
  }
  return;
}


/* [VTable] FlashExternalWindowModule virtual function #6
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void __thiscall FlashExternalWindowModule__vfunc_6(void *this,int param_1)

{
  undefined4 local_8;
  uint local_4;
  
  if (*(int *)((int)this + 0x24) != 0) {
    local_4 = (uint)*(ushort *)(param_1 + 0xc);
    local_8 = 0xd;
    (**(code **)(**(int **)(*(int *)((int)this + 0x24) + 0x34) + 0x84))(&local_8);
  }
  return;
}


/* [VTable] FlashExternalWindowModule virtual function #7
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void __fastcall FlashExternalWindowModule__vfunc_7(int param_1)

{
  undefined4 local_10;
  undefined4 local_c;
  undefined1 local_8;
  undefined1 local_7;
  undefined4 local_4;
  
  if (*(int *)(param_1 + 0x24) != 0) {
    local_c = FUN_0093acf0(param_1);
    local_10 = 5;
    local_7 = 0;
    local_4 = 0;
    local_8 = 0;
    (**(code **)(**(int **)(*(int *)(param_1 + 0x24) + 0x34) + 0x84))(&local_10);
  }
  return;
}


/* [VTable] FlashExternalWindowModule virtual function #8
   VTable: vtable_FlashExternalWindowModule at 018f9cc4 */

void __fastcall FlashExternalWindowModule__vfunc_8(int param_1)

{
  undefined4 local_10;
  undefined4 local_c;
  undefined1 local_8;
  undefined1 local_7;
  undefined4 local_4;
  
  if (*(int *)(param_1 + 0x24) != 0) {
    local_c = FUN_0093acf0(param_1);
    local_10 = 6;
    local_7 = 0;
    local_4 = 0;
    local_8 = 0;
    (**(code **)(**(int **)(*(int *)(param_1 + 0x24) + 0x34) + 0x84))(&local_10);
  }
  return;
}


/* Also in vtable: FlashExternalWindowModule (slot 0) */

undefined4 * __thiscall FlashExternalWindowModule__vfunc_0(void *this,byte param_1)

{
  FUN_0093b6f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== MissingAssetAppearanceJob.c ========== */

/*
 * SGW.exe - MissingAssetAppearanceJob (1 functions)
 */

undefined4 * __thiscall MissingAssetAppearanceJob__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01710888;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  FUN_00e9bc70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== MoviePlayer.c ========== */

/*
 * SGW.exe - MoviePlayer (1 functions)
 */

undefined4 * __thiscall MoviePlayer__vfunc_0(void *this,byte param_1)

{
  FUN_00de78e0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== PortraitManager.c ========== */

/*
 * SGW.exe - PortraitManager (1 functions)
 */

undefined4 * __thiscall PortraitManager__vfunc_0(void *this,byte param_1)

{
  FUN_00e6b810(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ScriptMapLayer.c ========== */

/*
 * SGW.exe - ScriptMapLayer (1 functions)
 */

undefined4 * __thiscall ScriptMapLayer__vfunc_0(void *this,byte param_1)

{
  FUN_00ae68b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ScriptedWindow.c ========== */

/*
 * SGW.exe - ScriptedWindow (1 functions)
 */

/* [VTable] ScriptedWindow virtual function #1
   VTable: vtable_ScriptedWindow at 018fadf4 */

void __fastcall ScriptedWindow__vfunc_1(int param_1)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int iStack_10;
  int iStack_c;
  
  iStack_c = **(int **)(param_1 + 0x14);
  iStack_10 = param_1 + 0x10;
  while( true ) {
    iVar3 = iStack_c;
    iVar2 = iStack_10;
    iVar1 = *(int *)(param_1 + 0x14);
    if (param_1 + 0x10 != iStack_10) {
      _invalid_parameter_noinfo();
    }
    if (iVar1 == iVar3) break;
    if (iVar2 == 0) {
      _invalid_parameter_noinfo();
    }
    if (iVar3 == *(int *)(iVar2 + 4)) {
      _invalid_parameter_noinfo();
    }
    FUN_011adcc0(*(int *)(iVar3 + 0x2c));
    FUN_00d07850(&iStack_10);
  }
  FUN_0094bf60(*(int *)(*(int *)(param_1 + 0x14) + 4));
  *(int *)(*(int *)(param_1 + 0x14) + 4) = *(int *)(param_1 + 0x14);
  *(undefined4 *)(param_1 + 0x18) = 0;
  *(undefined4 *)*(undefined4 *)(param_1 + 0x14) = *(undefined4 *)(param_1 + 0x14);
  *(int *)(*(int *)(param_1 + 0x14) + 8) = *(int *)(param_1 + 0x14);
  iStack_c = **(int **)(param_1 + 0x20);
  iStack_10 = param_1 + 0x1c;
  while( true ) {
    iVar3 = iStack_c;
    iVar2 = iStack_10;
    iVar1 = *(int *)(param_1 + 0x20);
    if (param_1 + 0x1c != iStack_10) {
      _invalid_parameter_noinfo();
    }
    if (iVar1 == iVar3) break;
    if (iVar2 == 0) {
      _invalid_parameter_noinfo();
    }
    if (iVar3 == *(int *)(iVar2 + 4)) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)(iVar3 + 0x2c) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)(iVar3 + 0x2c))(1);
    }
    FUN_00e6bd10(&iStack_10);
  }
  FUN_0094ca10(*(int *)(*(int *)(param_1 + 0x20) + 4));
  *(int *)(*(int *)(param_1 + 0x20) + 4) = *(int *)(param_1 + 0x20);
  *(undefined4 *)(param_1 + 0x24) = 0;
  *(undefined4 *)*(undefined4 *)(param_1 + 0x20) = *(undefined4 *)(param_1 + 0x20);
  *(int *)(*(int *)(param_1 + 0x20) + 8) = *(int *)(param_1 + 0x20);
  iStack_c = **(int **)(param_1 + 0x2c);
  iStack_10 = param_1 + 0x28;
  while( true ) {
    iVar3 = iStack_c;
    iVar2 = iStack_10;
    iVar1 = *(int *)(param_1 + 0x2c);
    if (param_1 + 0x28 != iStack_10) {
      _invalid_parameter_noinfo();
    }
    if (iVar1 == iVar3) break;
    if (iVar2 == 0) {
      _invalid_parameter_noinfo();
    }
    if (iVar3 == *(int *)(iVar2 + 4)) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)(iVar3 + 0x14) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)(iVar3 + 0x14))(1);
    }
    FUN_00e9a300(&iStack_10);
  }
  FUN_0094e2d0(*(int *)(*(int *)(param_1 + 0x2c) + 4));
  *(int *)(*(int *)(param_1 + 0x2c) + 4) = *(int *)(param_1 + 0x2c);
  *(undefined4 *)(param_1 + 0x30) = 0;
  *(undefined4 *)*(undefined4 *)(param_1 + 0x2c) = *(undefined4 *)(param_1 + 0x2c);
  *(int *)(*(int *)(param_1 + 0x2c) + 8) = *(int *)(param_1 + 0x2c);
  iStack_c = **(int **)(param_1 + 0x38);
  iStack_10 = param_1 + 0x34;
  while( true ) {
    iVar3 = iStack_c;
    iVar2 = iStack_10;
    iVar1 = *(int *)(param_1 + 0x38);
    if (param_1 + 0x34 != iStack_10) {
      _invalid_parameter_noinfo();
    }
    if (iVar1 == iVar3) break;
    if (iVar2 == 0) {
      _invalid_parameter_noinfo();
    }
    if (iVar3 == *(int *)(iVar2 + 4)) {
      _invalid_parameter_noinfo();
    }
    if (*(undefined4 **)(iVar3 + 0x2c) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)(iVar3 + 0x2c))(1);
    }
    FUN_00e6bd10(&iStack_10);
  }
  FUN_0094ca10(*(int *)(*(int *)(param_1 + 0x38) + 4));
  *(int *)(*(int *)(param_1 + 0x38) + 4) = *(int *)(param_1 + 0x38);
  *(undefined4 *)(param_1 + 0x3c) = 0;
  *(undefined4 *)*(undefined4 *)(param_1 + 0x38) = *(undefined4 *)(param_1 + 0x38);
  *(int *)(*(int *)(param_1 + 0x38) + 8) = *(int *)(param_1 + 0x38);
  return;
}




/* ========== ScriptedWindow_ActionEventHandler.c ========== */

/*
 * SGW.exe - ScriptedWindow_ActionEventHandler (1 functions)
 */

/* Also in vtable: ScriptedWindow_ActionEventHandler (slot 0) */

undefined4 * __thiscall ScriptedWindow_ActionEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_009482b0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ScriptedWindow_BaseHandler.c ========== */

/*
 * SGW.exe - ScriptedWindow_BaseHandler (1 functions)
 */

/* Also in vtable: ScriptedWindow_BaseHandler (slot 0) */

undefined4 * __thiscall ScriptedWindow_BaseHandler__vfunc_0(void *this,byte param_1)

{
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== ScriptedWindow_CEGUI_VActivationEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VActivationEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VActivationEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VActivationEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::ActivationEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VDragDropEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VDragDropEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VDragDropEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VDragDropEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::DragDropEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VHeaderSequenceEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VHeaderSequenceEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VHeaderSequenceEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VHeaderSequenceEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this =
       ScriptedWindow::UIEventHandler<class_CEGUI::HeaderSequenceEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VKeyEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VKeyEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VKeyEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VKeyEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::KeyEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VMouseEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VMouseEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VMouseEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VMouseEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::MouseEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VRichTextEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VRichTextEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VRichTextEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VRichTextEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::RichTextEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VTreeEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VTreeEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VTreeEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VTreeEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::TreeEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VUpdateEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VUpdateEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VUpdateEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VUpdateEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::UpdateEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CEGUI_VWindowEventArgs.c ========== */

/*
 * SGW.exe - ScriptedWindow_CEGUI_VWindowEventArgs (1 functions)
 */

/* Also in vtable: ScriptedWindow_CEGUI_VWindowEventArgs___UIEventHandler (slot 0) */

undefined4 * __thiscall
ScriptedWindow_CEGUI_VWindowEventArgs___UIEventHandler__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_016f9058;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = ScriptedWindow::UIEventHandler<class_CEGUI::WindowEventArgs>::vftable;
  local_4 = 0xffffffff;
  FUN_00946020(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}




/* ========== ScriptedWindow_CommandEventHandler.c ========== */

/*
 * SGW.exe - ScriptedWindow_CommandEventHandler (1 functions)
 */

/* Also in vtable: ScriptedWindow_CommandEventHandler (slot 0) */

undefined4 * __thiscall ScriptedWindow_CommandEventHandler__vfunc_0(void *this,byte param_1)

{
  FUN_0094a490(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== StaticMeshAppearanceJob.c ========== */

/*
 * SGW.exe - StaticMeshAppearanceJob (1 functions)
 */

undefined4 * __thiscall StaticMeshAppearanceJob__vfunc_0(void *this,byte param_1)

{
  FUN_00e9bc70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== SynchronousCompositor_Continuation.c ========== */

/*
 * SGW.exe - SynchronousCompositor_Continuation (1 functions)
 */

undefined4 * __thiscall SynchronousCompositor_Continuation__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_017126d8;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = ICompositingProcessContinuation::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== UCodecMovie.c ========== */

/*
 * SGW.exe - UCodecMovie (4 functions)
 */

/* [VTable] UCodecMovie virtual function #70
   VTable: vtable_UCodecMovie at 0184bf6c */

undefined4 UCodecMovie__vfunc_70(void)

{
  return 0;
}


/* Also in vtable: UCodecMovie (slot 0) */

undefined4 UCodecMovie__vfunc_0(void)

{
  return 1;
}


/* [VTable] UCodecMovie virtual function #1
   VTable: vtable_UCodecMovie at 0184bf6c */

bool __fastcall UCodecMovie__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UCodecMovie virtual function #2
   VTable: vtable_UCodecMovie at 0184bf6c */

int * __thiscall UCodecMovie__vfunc_2(void *this,byte param_1)

{
  FUN_005f3090(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UCodecMovieBink.c ========== */

/*
 * SGW.exe - UCodecMovieBink (20 functions)
 */

/* [VTable] UCodecMovieBink virtual function #72
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __fastcall UCodecMovieBink__vfunc_72(int param_1)

{
                    /* WARNING: Could not recover jumptable at 0x005f0d68. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(**(int **)(param_1 + 0x60) + 0x14))();
  return;
}


/* [VTable] UCodecMovieBink virtual function #67
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 UCodecMovieBink__vfunc_67(void)

{
  return 1;
}


/* [VTable] UCodecMovieBink virtual function #68
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 __fastcall UCodecMovieBink__vfunc_68(int param_1)

{
  undefined4 uVar1;
  
  uVar1 = 1;
  if (*(undefined4 **)(param_1 + 0x48) != (undefined4 *)0x0) {
    uVar1 = **(undefined4 **)(param_1 + 0x48);
  }
  return uVar1;
}


/* [VTable] UCodecMovieBink virtual function #69
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 __fastcall UCodecMovieBink__vfunc_69(int param_1)

{
  undefined4 uVar1;
  
  uVar1 = 1;
  if (*(int *)(param_1 + 0x48) != 0) {
    uVar1 = *(undefined4 *)(*(int *)(param_1 + 0x48) + 4);
  }
  return uVar1;
}


/* [VTable] UCodecMovieBink virtual function #70
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 UCodecMovieBink__vfunc_70(void)

{
  return 2;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] UCodecMovieBink virtual function #73
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 __thiscall UCodecMovieBink__vfunc_73(void *this,int param_1)

{
  float fVar1;
  int iVar2;
  undefined4 uVar3;
  float10 fVar4;
  
  uVar3 = 0;
  if (*(int *)((int)this + 0x48) != 0) {
    (**(code **)(*(int *)this + 300))();
  }
  if (param_1 != 0) {
    *(int *)((int)this + 0x50) = param_1;
    _BinkSetSoundTrack_8(0,0);
    iVar2 = _BinkOpen_8(*(undefined4 *)((int)this + 0x50),0x4000400);
    *(int *)((int)this + 0x48) = iVar2;
    if (iVar2 != 0) {
      fVar1 = (float)*(int *)(iVar2 + 8);
      uVar3 = 1;
      if (*(int *)(iVar2 + 8) < 0) {
        fVar1 = fVar1 + _DAT_01810160;
      }
      fVar4 = (float10)(**(code **)(*(int *)this + 0x11c))();
      *(float *)((int)this + 0x3c) = (float)((float10)fVar1 / fVar4);
    }
  }
  return uVar3;
}


/* [VTable] UCodecMovieBink virtual function #76
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __fastcall UCodecMovieBink__vfunc_76(int param_1)

{
  if (*(int *)(param_1 + 0x48) != 0) {
    _BinkGoto_12(*(int *)(param_1 + 0x48),1,0);
  }
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] UCodecMovieBink virtual function #71
   VTable: vtable_UCodecMovieBink at 0184c324 */

float10 __fastcall UCodecMovieBink__vfunc_71(int param_1)

{
  int iVar1;
  uint uVar2;
  float10 fVar3;
  
  iVar1 = *(int *)(param_1 + 0x48);
  if (iVar1 == 0) {
    fVar3 = (float10)0.0;
  }
  else {
    uVar2 = *(uint *)(iVar1 + 0x14) / *(uint *)(iVar1 + 0x18);
    fVar3 = (float10)(int)uVar2;
    if ((int)uVar2 < 0) {
      return fVar3 + (float10)_DAT_01810160;
    }
  }
  return fVar3;
}


/* [VTable] UCodecMovieBink virtual function #79
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __thiscall UCodecMovieBink__vfunc_79(void *this,undefined4 param_1,undefined4 param_2)

{
  if (*(int *)((int)this + 0x48) != 0) {
    _BinkPause_8(*(int *)((int)this + 0x48),0);
    (**(code **)(**(int **)((int)this + 0x60) + 0xc))();
    *(undefined4 *)((int)this + 0x54) = param_2;
    *(undefined4 *)((int)this + 0x58) = param_1;
  }
  return;
}


/* [VTable] UCodecMovieBink virtual function #80
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __thiscall UCodecMovieBink__vfunc_80(void *this,int param_1)

{
  if (*(int *)((int)this + 0x48) != 0) {
    _BinkPause_8(*(int *)((int)this + 0x48),param_1);
    if (param_1 != 0) {
      (**(code **)(**(int **)((int)this + 0x60) + 8))();
      return;
    }
    (**(code **)(**(int **)((int)this + 0x60) + 0xc))();
  }
  return;
}


/* [VTable] UCodecMovieBink virtual function #81
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __fastcall UCodecMovieBink__vfunc_81(int param_1)

{
  if (*(int *)(param_1 + 0x48) != 0) {
    _BinkPause_8(*(int *)(param_1 + 0x48),1);
    _BinkGoto_12(*(undefined4 *)(param_1 + 0x48),1,0);
                    /* WARNING: Could not recover jumptable at 0x005f0f4a. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (**(code **)(**(int **)(param_1 + 0x60) + 8))();
    return;
  }
  return;
}


/* Also in vtable: UCodecMovieBink (slot 0) */

undefined4 UCodecMovieBink__vfunc_0(void)

{
  return 1;
}


/* [VTable] UCodecMovieBink virtual function #9
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 __fastcall UCodecMovieBink__vfunc_9(int param_1)

{
  int iVar1;
  
  if (*(undefined4 **)(param_1 + 0x5c) != (undefined4 *)0x0) {
    iVar1 = FUN_0054bfc0(*(undefined4 **)(param_1 + 0x5c));
    if (iVar1 == 0) {
      return 1;
    }
  }
  return 0;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] UCodecMovieBink virtual function #74
   VTable: vtable_UCodecMovieBink at 0184c324 */

undefined4 __thiscall
UCodecMovieBink__vfunc_74(void *this,undefined4 *param_1,uint param_2,uint param_3)

{
  float fVar1;
  undefined **lpFileName;
  HANDLE hFile;
  BOOL BVar2;
  int iVar3;
  DWORD unaff_EDI;
  float10 fVar4;
  LARGE_INTEGER LStack_8;
  
  if (*(int *)((int)this + 0x48) != 0) {
    (**(code **)(*(int *)this + 300))();
  }
  if (param_1[1] == 0) {
    lpFileName = &PTR_017f8e10;
  }
  else {
    lpFileName = (undefined **)*param_1;
  }
  hFile = CreateFileW((LPCWSTR)lpFileName,0x80000000,1,(LPSECURITY_ATTRIBUTES)0x0,3,0x80,(HANDLE)0x0
                     );
  *(HANDLE *)((int)this + 0x4c) = hFile;
  if (hFile == (HANDLE)0xffffffff) {
    GetLastError();
    return 0;
  }
  LStack_8.s.LowPart = 0;
  LStack_8.s.HighPart = 0;
  BVar2 = GetFileSizeEx(hFile,&LStack_8);
  if (BVar2 != 0) {
    if ((LStack_8.s.HighPart <= (int)(uint)CARRY4(param_3,param_2)) &&
       ((LStack_8.s.HighPart < (int)(uint)CARRY4(param_3,param_2) ||
        (LStack_8.s.LowPart < param_3 + param_2)))) {
      CloseHandle(*(HANDLE *)((int)this + 0x4c));
      *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
      return 0;
    }
    BVar2 = SetFilePointerEx(*(HANDLE *)((int)this + 0x4c),(LARGE_INTEGER)0x0,(PLARGE_INTEGER)0x0,
                             unaff_EDI);
    if (BVar2 != 0) {
      _BinkSetSoundTrack_8(0,0);
      iVar3 = _BinkOpen_8(*(undefined4 *)((int)this + 0x4c),0x800400);
      *(int *)((int)this + 0x48) = iVar3;
      if (iVar3 != 0) {
        fVar1 = (float)*(int *)(iVar3 + 8);
        if (*(int *)(iVar3 + 8) < 0) {
          fVar1 = fVar1 + _DAT_01810160;
        }
        fVar4 = (float10)(**(code **)(*(int *)this + 0x11c))();
        *(float *)((int)this + 0x3c) = (float)((float10)fVar1 / fVar4);
        return 1;
      }
      CloseHandle(*(HANDLE *)((int)this + 0x4c));
      *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
      return 0;
    }
  }
  GetLastError();
  CloseHandle(*(HANDLE *)((int)this + 0x4c));
  *(undefined4 *)((int)this + 0x4c) = 0xffffffff;
  return 0;
}


/* [VTable] UCodecMovieBink virtual function #1
   VTable: vtable_UCodecMovieBink at 0184c324 */

bool __fastcall UCodecMovieBink__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2c40 == (undefined4 *)0x0) {
    DAT_01ee2c40 = FUN_005f1320();
    FUN_005f1020();
  }
  return puVar1 != DAT_01ee2c40;
}


/* [VTable] UCodecMovieBink virtual function #77
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __thiscall UCodecMovieBink__vfunc_77(void *this,int *param_1)

{
  FUN_005f29d0((int)this);
  FUN_005f2b00(this,param_1);
  return;
}


/* [VTable] UCodecMovieBink virtual function #82
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __fastcall UCodecMovieBink__vfunc_82(int *param_1)

{
  int iVar1;
  
  if (param_1[0x11] != 0) {
    (**(code **)(*(int *)param_1[0x10] + 0x14))();
    iVar1 = param_1[0x10];
    if (iVar1 != 0) {
      UCodecMovieBink__unknown_0050c540(iVar1);
                    /* WARNING: Subroutine does not return */
      scalable_free(iVar1);
    }
    param_1[0x10] = 0;
    param_1[0x11] = 0;
  }
                    /* WARNING: Could not recover jumptable at 0x005f2d65. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*param_1 + 0x130))();
  return;
}


/* [VTable] UCodecMovieBink virtual function #75
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __fastcall UCodecMovieBink__vfunc_75(int param_1)

{
  int iVar1;
  int iStack_2c;
  int *piStack_28;
  undefined **ppuStack_24;
  int iStack_20;
  int aiStack_1c [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_01693993;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (*(int *)(param_1 + 0x48) != 0) {
    ExceptionList = &pvStack_c;
    _BinkClose_4(*(int *)(param_1 + 0x48));
    *(undefined4 *)(param_1 + 0x48) = 0;
  }
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/UnCodecBink.inl"
                 ,0x19f);
  }
  if (DAT_01ee2618 == 0) {
    ppuStack_24 = `public:_virtual_void___thiscall_UCodecMovieBink::Close(void)'::__l5::FreeInternal
                  ::vftable;
    uStack_4 = 3;
    iStack_20 = param_1;
    if (*(int *)(param_1 + 0x44) != 0) {
      (**(code **)(**(int **)(param_1 + 0x40) + 0x14))();
      iVar1 = *(int *)(param_1 + 0x40);
      if (iVar1 != 0) {
        UCodecMovieBink__unknown_0050c540(iVar1);
                    /* WARNING: Subroutine does not return */
        scalable_free(iVar1);
      }
      *(undefined4 *)(param_1 + 0x40) = 0;
      *(undefined4 *)(param_1 + 0x44) = 0;
    }
    uStack_4 = 0xffffffff;
    ppuStack_24 = FRenderCommand::vftable;
  }
  else {
    piStack_28 = FUN_00419650(aiStack_1c,&DAT_01ee2634,8);
    ppuStack_24 = (undefined **)piStack_28[2];
    uStack_4 = 1;
    if (ppuStack_24 != (undefined **)0x0) {
      iStack_2c = param_1;
      FUN_005f1240(ppuStack_24,&iStack_2c);
    }
    uStack_4 = 0xffffffff;
    FUN_00419230(aiStack_1c);
  }
  iVar1 = *(int *)(param_1 + 0x50);
  if (iVar1 != 0) {
    if (DAT_01ea5778 == (int *)0x0) {
      FUN_00416660();
    }
    (**(code **)(*DAT_01ea5778 + 0xc))(iVar1);
    *(undefined4 *)(param_1 + 0x50) = 0;
  }
  if (*(HANDLE *)(param_1 + 0x4c) != (HANDLE)0xffffffff) {
    CloseHandle(*(HANDLE *)(param_1 + 0x4c));
    *(undefined4 *)(param_1 + 0x4c) = 0xffffffff;
  }
  *(undefined4 *)(param_1 + 0x3c) = 0;
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] UCodecMovieBink virtual function #8
   VTable: vtable_UCodecMovieBink at 0184c324 */

void __fastcall UCodecMovieBink__vfunc_8(int *param_1)

{
  int iVar1;
  undefined4 *puVar2;
  int *piStack_38;
  undefined **ppuStack_34;
  int *piStack_30;
  undefined4 *puStack_2c;
  exception aeStack_28 [12];
  int aiStack_1c [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016939d9;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  UTestIpDrv__vfunc_8(param_1);
  (**(code **)(*(int *)param_1[0x18] + 8))();
  iVar1 = FUN_0054bb60();
  if (iVar1 == 0) {
    FUN_00486000("IsInGameThread()",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\engine\\src\\../Bink/Src/UnCodecBink.inl"
                 ,0x238);
  }
  if (DAT_01ee2618 == 0) {
    ppuStack_34 = `public:_virtual_void___thiscall_UCodecMovieBink::BeginDestroy(void)'::__l2::
                  FreeInternal::vftable;
    uStack_4 = 3;
    piStack_30 = param_1;
    if (param_1[0x11] != 0) {
      (**(code **)(*(int *)param_1[0x10] + 0x14))();
      iVar1 = param_1[0x10];
      if (iVar1 != 0) {
        UCodecMovieBink__unknown_0050c540(iVar1);
                    /* WARNING: Subroutine does not return */
        scalable_free(iVar1);
      }
      param_1[0x10] = 0;
      param_1[0x11] = 0;
    }
    uStack_4 = 0xffffffff;
    ppuStack_34 = FRenderCommand::vftable;
  }
  else {
    ppuStack_34 = (undefined **)FUN_00419650(aiStack_1c,&DAT_01ee2634,8);
    puStack_2c = (undefined4 *)ppuStack_34[2];
    uStack_4 = 1;
    if (puStack_2c != (undefined4 *)0x0) {
      piStack_38 = param_1;
      FUN_005f12a0(puStack_2c,&piStack_38);
    }
    uStack_4 = 0xffffffff;
    FUN_00419230(aiStack_1c);
  }
  if (param_1[0x17] == 0) {
    puVar2 = (undefined4 *)scalable_malloc(4);
    if (puVar2 == (undefined4 *)0x0) {
      FUN_004099f0(aeStack_28);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(aeStack_28,(ThrowInfo *)&DAT_01d65cc8);
    }
    *puVar2 = 0;
    uStack_4 = 0xffffffff;
    param_1[0x17] = (int)puVar2;
    puStack_2c = puVar2;
  }
  FUN_0054beb0((LONG *)param_1[0x17]);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] UCodecMovieBink virtual function #2
   VTable: vtable_UCodecMovieBink at 0184c324 */

int * __thiscall UCodecMovieBink__vfunc_2(void *this,byte param_1)

{
  FUN_005f3620(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UCodecMovieFallback.c ========== */

/*
 * SGW.exe - UCodecMovieFallback (12 functions)
 */

/* [VTable] UCodecMovieFallback virtual function #67
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

undefined4 UCodecMovieFallback__vfunc_67(void)

{
  return 1;
}


/* [VTable] UCodecMovieFallback virtual function #68
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

undefined4 UCodecMovieFallback__vfunc_68(void)

{
  return 1;
}


/* [VTable] UCodecMovieFallback virtual function #69
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

undefined4 UCodecMovieFallback__vfunc_69(void)

{
  return 1;
}


/* [VTable] UCodecMovieFallback virtual function #70
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

undefined4 UCodecMovieFallback__vfunc_70(void)

{
  return 2;
}


/* [VTable] UCodecMovieFallback virtual function #74
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

undefined4 __fastcall UCodecMovieFallback__vfunc_74(int param_1)

{
  *(undefined4 *)(param_1 + 0x3c) = DAT_017f7ea0;
  *(undefined4 *)(param_1 + 0x40) = 0;
  return 1;
}


/* [VTable] UCodecMovieFallback virtual function #73
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

undefined4 __fastcall UCodecMovieFallback__vfunc_73(int param_1)

{
  *(undefined4 *)(param_1 + 0x3c) = DAT_017f7ea0;
  *(undefined4 *)(param_1 + 0x40) = 0;
  return 1;
}


/* [VTable] UCodecMovieFallback virtual function #76
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

void __fastcall UCodecMovieFallback__vfunc_76(int param_1)

{
  *(undefined4 *)(param_1 + 0x40) = 0;
  return;
}


/* [VTable] UCodecMovieFallback virtual function #71
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

float10 UCodecMovieFallback__vfunc_71(void)

{
  return (float10)DAT_01834eb0;
}


/* Also in vtable: UCodecMovieFallback (slot 0) */

undefined4 UCodecMovieFallback__vfunc_0(void)

{
  return 1;
}


/* [VTable] UCodecMovieFallback virtual function #1
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

bool __fastcall UCodecMovieFallback__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2c40 == (undefined4 *)0x0) {
    DAT_01ee2c40 = FUN_005f1320();
    FUN_005f1020();
  }
  return puVar1 != DAT_01ee2c40;
}


/* [VTable] UCodecMovieFallback virtual function #77
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

void __thiscall UCodecMovieFallback__vfunc_77(void *this,int param_1)

{
  int *piVar1;
  int *piVar2;
  undefined4 uVar3;
  float10 fVar4;
  uint uVar5;
  undefined4 *puVar6;
  int iVar7;
  undefined4 uVar8;
  int iVar9;
  undefined4 uVar10;
  undefined4 uStack_38;
  float fStack_34;
  undefined4 uStack_30;
  undefined4 uStack_2c;
  int aiStack_28 [7];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0169382d;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  fVar4 = (float10)(**(code **)(*(int *)this + 0x11c))();
  fVar4 = (float10)1 / fVar4 + (float10)*(float *)((int)this + 0x40);
  *(float *)((int)this + 0x40) = (float)fVar4;
  if (*(float *)((int)this + 0x3c) < (float)fVar4) {
    *(undefined4 *)((int)this + 0x40) = 0;
  }
  if ((param_1 != 0) && ((*(byte *)(param_1 + 0x10) & 1) != 0)) {
    fStack_34 = *(float *)((int)this + 0x40) / *(float *)((int)this + 0x3c);
    uStack_38 = DAT_017f7ea0;
    uStack_30 = 0;
    uStack_2c = DAT_017f7ea0;
    piVar1 = (int *)FUN_005e9be0(aiStack_28);
    uStack_4 = 0;
    piVar2 = (int *)(*(code *)**(undefined4 **)(param_1 + 0x2c))();
    uVar3 = FUN_00ec47d0();
    FUN_00ec6530(uVar3,piVar2,piVar1);
    uStack_4 = 0xffffffff;
    FSceneRenderTargets__unknown_005d9c10(aiStack_28);
    uVar10 = 0;
    iVar9 = 0;
    uVar8 = 0;
    iVar7 = 0;
    puVar6 = &uStack_38;
    uVar5 = 1;
    uVar3 = FUN_00ec47d0();
    FUN_00ec6a20(uVar3,uVar5,puVar6,iVar7,uVar8,iVar9,uVar10);
    piVar1 = aiStack_28;
    uVar3 = 0;
    aiStack_28[0] = 0;
    aiStack_28[1] = 0;
    aiStack_28[2] = 0xffffffff;
    aiStack_28[3] = 0xffffffff;
    aiStack_28[4] = 0xffffffff;
    aiStack_28[5] = 0xffffffff;
    iVar7 = (*(code *)**(undefined4 **)(param_1 + 0x2c))();
    FUN_00ecba70(iVar7,uVar3,piVar1);
  }
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] UCodecMovieFallback virtual function #2
   VTable: vtable_UCodecMovieFallback at 0184c0bc */

int * __thiscall UCodecMovieFallback__vfunc_2(void *this,byte param_1)

{
  FUN_005f31c0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UFlashMovieFactoryNew.c ========== */

/*
 * SGW.exe - UFlashMovieFactoryNew (3 functions)
 */

/* WARNING: Variable defined which should be unmapped: param_1 */
/* [VTable] UFlashMovieFactoryNew virtual function #81
   VTable: vtable_UFlashMovieFactoryNew at 01960a8c */

void UFlashMovieFactoryNew__vfunc_81(int param_1)

{
  int iVar1;
  int *piVar2;
  int iVar3;
  code *pcVar4;
  int *unaff_ESI;
  int unaff_EDI;
  wchar_t *unaff_retaddr;
  int in_stack_00000014;
  int in_stack_00000018;
  void *in_stack_00000030;
  undefined4 in_stack_00000038;
  int in_stack_00000040;
  
  piVar2 = FUN_0049e960(&stack0x0000001c,unaff_retaddr,param_1);
  iVar3 = *piVar2;
  iVar1 = piVar2[1];
  FUN_0049e960(&stack0x00000014,L"ImageDataSource",1);
  if ((iVar3 == in_stack_00000014) && (iVar1 == in_stack_00000018)) {
    pcVar4 = *(code **)(unaff_ESI[0xca] + 0xc);
  }
  else {
    FUN_0049e960(&stack0x00000014,L"ImageComponent",1);
    if ((iVar3 != in_stack_00000014) || (iVar1 != in_stack_00000018)) goto LAB_006501ef;
    iVar3 = **(int **)(in_stack_00000040 + 8);
    if (unaff_EDI == iVar3) {
      if (unaff_ESI[0xd6] != 0) {
        iVar3 = FUN_00659750((int)unaff_ESI);
        iVar3 = *(int *)(iVar3 + 0x358);
        if (iVar3 != 0) {
          iVar1 = unaff_ESI[0xd6];
          *(undefined4 *)(iVar1 + 0x74) = *(undefined4 *)(iVar3 + 0x74);
          *(undefined4 *)(iVar1 + 0x78) = *(undefined4 *)(iVar3 + 0x78);
        }
        FUN_00656280(&stack0x00000014,unaff_ESI[0xd6]);
        FUN_0063f630(unaff_ESI,&stack0x00000014);
        FUN_00667f90(unaff_ESI[0xd6]);
        FUN_0066bb30((int *)unaff_ESI[0xd6]);
      }
      goto LAB_006501ef;
    }
    if (unaff_ESI[0xd6] == 0) goto LAB_006501ef;
    FUN_0049e960(&stack0x0000001c,L"ImageRef",1);
    if (*(int *)(iVar3 + 4) == -1) {
      piVar2 = FUN_0049e960(&stack0x00000024,L"<uninitialized>",1);
    }
    else {
      piVar2 = (int *)(iVar3 + 0x2c);
    }
    in_stack_00000014 = *piVar2;
    in_stack_00000018 = piVar2[1];
    iVar3 = FUN_00497f80(&stack0x00000014,(int *)&stack0x0000001c);
    if (iVar3 == 0) goto LAB_006501ef;
    iVar3 = FUN_00667f90(unaff_ESI[0xd6]);
    if (iVar3 == 0) goto LAB_006501ef;
    FUN_00667f90(unaff_ESI[0xd6]);
    FUN_0041aab0(&stack0x00000024,(short *)&DAT_01856a34);
    in_stack_00000038 = 0;
    (**(code **)(unaff_ESI[0xca] + 4))(&stack0x00000024);
    in_stack_00000038 = 0xffffffff;
    FUN_0041b420((int *)&stack0x00000024);
    pcVar4 = *(code **)(*unaff_ESI + 600);
  }
  (*pcVar4)();
LAB_006501ef:
  UUIObject__vfunc_18(unaff_ESI,in_stack_00000040);
  ExceptionList = in_stack_00000030;
  return;
}


/* [VTable] UFlashMovieFactoryNew virtual function #1
   VTable: vtable_UFlashMovieFactoryNew at 01960a8c */

bool __fastcall UFlashMovieFactoryNew__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee11dc == (undefined4 *)0x0) {
    DAT_01ee11dc = FUN_00500b90();
    FUN_00500940();
  }
  return puVar1 != DAT_01ee11dc;
}


/* [VTable] UFlashMovieFactoryNew virtual function #2
   VTable: vtable_UFlashMovieFactoryNew at 01960a8c */

int * __thiscall UFlashMovieFactoryNew__vfunc_2(void *this,byte param_1)

{
  FUN_00aef030(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== ULoadingScreenManager.c ========== */

/*
 * SGW.exe - ULoadingScreenManager (4 functions)
 */

/* [VTable] ULoadingScreenManager virtual function #68
   VTable: vtable_ULoadingScreenManager at 018eb6cc */

void __fastcall ULoadingScreenManager__vfunc_68(int param_1)

{
  *(uint *)(param_1 + 0x3c) = *(uint *)(param_1 + 0x3c) & 0xfffffffe;
  return;
}


/* Also in vtable: ULoadingScreenManager (slot 0) */

undefined4 ULoadingScreenManager__vfunc_0(void)

{
  return 0;
}


/* [VTable] ULoadingScreenManager virtual function #1
   VTable: vtable_ULoadingScreenManager at 018eb6cc */

bool __fastcall ULoadingScreenManager__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] ULoadingScreenManager virtual function #2
   VTable: vtable_ULoadingScreenManager at 018eb6cc */

int * __thiscall ULoadingScreenManager__vfunc_2(void *this,byte param_1)

{
  FUN_008d4480(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== VisualMesh.c ========== */

/*
 * SGW.exe - VisualMesh (1 functions)
 */

undefined4 * __thiscall VisualMesh__vfunc_0(void *this,byte param_1)

{
  FUN_01188120(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}



