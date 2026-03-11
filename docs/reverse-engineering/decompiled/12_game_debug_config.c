/*
 * SGW.exe Decompilation - 12_game_debug_config
 * Classes: 37
 * Stargate Worlds Client
 */


/* ========== CriticalErrorHandler.c ========== */

/*
 * SGW.exe - CriticalErrorHandler (1 functions)
 */

/* Also in vtable: CriticalErrorHandler (slot 0) */

undefined4 * __thiscall CriticalErrorHandler__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = CriticalErrorHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== CriticalMessageCallback.c ========== */

/*
 * SGW.exe - CriticalMessageCallback (1 functions)
 */

/* [VTable] CriticalMessageCallback virtual function #1
   VTable: vtable_CriticalMessageCallback at 018cae88 */

undefined4 * __thiscall CriticalMessageCallback__vfunc_1(void *this,byte param_1)

{
  *(undefined ***)this = CriticalMessageCallback::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DSBreakOnChange.c ========== */

/*
 * SGW.exe - DSBreakOnChange (9 functions)
 */

/* [VTable] DSBreakOnChange virtual function #7
   VTable: vtable_DSBreakOnChange at 01910fb8 */

void __thiscall DSBreakOnChange__vfunc_7(void *this,undefined4 param_1)

{
  if (*(int **)((int)this + 0x18) != (int *)0x0) {
                    /* WARNING: Could not recover jumptable at 0x009f2b1e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (**(code **)(**(int **)((int)this + 0x18) + 0x1c))();
    return;
  }
  *(undefined4 *)((int)this + 4) = param_1;
  return;
}


/* [VTable] DSBreakOnChange virtual function #6
   VTable: vtable_DSBreakOnChange at 01910fb8 */

void __fastcall DSBreakOnChange__vfunc_6(int param_1)

{
  if (*(int *)(param_1 + 0x18) != 0) {
                    /* WARNING: Could not recover jumptable at 0x009f2b40. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (**(code **)(**(int **)(param_1 + 0x18) + 0x18))();
    return;
  }
  return;
}


/* [VTable] DSBreakOnChange virtual function #1
   VTable: vtable_DSBreakOnChange at 01910fb8 */

undefined4 __fastcall DSBreakOnChange__vfunc_1(int param_1)

{
  undefined4 uVar1;
  
  if (*(int *)(param_1 + 0x18) != 0) {
                    /* WARNING: Could not recover jumptable at 0x009f2b5e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    uVar1 = (**(code **)(**(int **)(param_1 + 0x18) + 4))();
    return uVar1;
  }
  return *(undefined4 *)(param_1 + 4);
}


/* [VTable] DSBreakOnChange virtual function #4
   VTable: vtable_DSBreakOnChange at 01910fb8 */

undefined4 __thiscall DSBreakOnChange__vfunc_4(void *this,int param_1)

{
  int iVar1;
  
  if (param_1 == 0) {
    return 0;
  }
  if (*(int **)((int)this + 0x18) != (int *)0x0) {
    iVar1 = (**(code **)(**(int **)((int)this + 0x18) + 0x10))(param_1);
    if (iVar1 != 0) {
      return 1;
    }
    if (*(undefined4 **)((int)this + 0x18) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)((int)this + 0x18))(1);
    }
  }
  *(int *)((int)this + 0x18) = param_1;
  return 1;
}


/* [VTable] DSBreakOnChange virtual function #5
   VTable: vtable_DSBreakOnChange at 01910fb8 */

bool __thiscall DSBreakOnChange__vfunc_5(void *this,int param_1)

{
  int iVar1;
  
  if (((param_1 != 0) && (*(int *)((int)this + 0x18) != 0)) && ((void *)param_1 != this)) {
    iVar1 = (**(code **)(**(int **)((int)this + 0x18) + 0x14))(param_1);
    if (iVar1 != 0) {
      return true;
    }
    return param_1 == *(int *)((int)this + 0x18);
  }
  return false;
}


/* [VTable] DSBreakOnChange virtual function #3
   VTable: vtable_DSBreakOnChange at 01910fb8 */

void __fastcall DSBreakOnChange__vfunc_3(int param_1)

{
  *(undefined4 *)(param_1 + 0x1c) = 0;
  if (*(int **)(param_1 + 0x18) != (int *)0x0) {
                    /* WARNING: Could not recover jumptable at 0x009f2c23. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (**(code **)(**(int **)(param_1 + 0x18) + 0xc))();
    return;
  }
  return;
}


/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */
/* [VTable] DSBreakOnChange virtual function #2
   VTable: vtable_DSBreakOnChange at 01910fb8 */

void __thiscall DSBreakOnChange__vfunc_2(void *this,undefined4 param_1)

{
  bool bVar1;
  int iVar2;
  int iVar3;
  undefined4 *puVar4;
  undefined3 extraout_var;
  BOOL BVar5;
  
  if (*(int *)((int)this + 8) == 0) {
    FUN_00486000("Debugger",".\\Debugger\\UnDebuggerCore.cpp",0x620);
  }
  iVar2 = (**(code **)(*(int *)this + 4))();
  if (iVar2 == 0) {
    FUN_00486000("Node",".\\Debugger\\UnDebuggerCore.cpp",0x623);
  }
  if (((*(int *)(*(int *)((int)this + 8) + 0x58) != 0) ||
      (**(int **)(*(int *)((int)this + 8) + 0x84) == 0)) ||
     (iVar3 = (**(code **)(*(int *)this + 0x20))(param_1), iVar3 == 0)) {
    if (*(int *)((int)this + 0x18) != 0) {
      (**(code **)(**(int **)((int)this + 0x18) + 8))(param_1);
    }
    return;
  }
  if (((iVar2 != 0) && (*(int *)(iVar2 + 4) != 0)) && (*(int *)(*(int *)(iVar2 + 4) + 4) != 0)) {
    puVar4 = UClass__unknown_00419340();
    bVar1 = FUN_00419370(*(void **)(*(int *)(iVar2 + 4) + 4),(int)puVar4);
    if (CONCAT31(extraout_var,bVar1) != 0) {
      BVar5 = IsDebuggerPresent();
      if (BVar5 == 0) {
        return;
      }
      BVar5 = IsDebuggerPresent();
      if (BVar5 == 0) {
        return;
      }
      _DAT_00000003 = 0xd;
      return;
    }
  }
  FUN_009f3390(*(int *)((int)this + 8));
  return;
}


/* [VTable] DSBreakOnChange virtual function #8
   VTable: vtable_DSBreakOnChange at 01910fb8 */

void __fastcall DSBreakOnChange__vfunc_8(int *param_1)

{
  int iVar1;
  
  if (param_1[5] == 0) {
    FUN_00486000("Watch",".\\Debugger\\UnDebuggerCore.cpp",0x6b4);
  }
  iVar1 = (**(code **)(*(int *)param_1[5] + 0xc))();
  if (iVar1 != 0) {
    param_1[7] = 1;
    return;
  }
  if ((int *)param_1[6] != (int *)0x0) {
                    /* WARNING: Could not recover jumptable at 0x009f4b57. Too many branches */
                    /* WARNING: Treating indirect jump as call */
    (**(code **)(*(int *)param_1[6] + 0x20))();
    return;
  }
  FDebuggerState__vfunc_8(param_1);
  return;
}


/* Also in vtable: DSBreakOnChange (slot 0) */

undefined4 * __thiscall DSBreakOnChange__vfunc_0(void *this,byte param_1)

{
  FUN_009f2ab0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DSCStuff.c ========== */

/*
 * SGW.exe - DSCStuff (1 functions)
 */

/* Also in vtable: DSCStuff (slot 0) */

undefined4 * __thiscall DSCStuff__vfunc_0(void *this,byte param_1)

{
  FUN_0045f4f0(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DSStepInto.c ========== */

/*
 * SGW.exe - DSStepInto (1 functions)
 */

/* [VTable] DSStepInto virtual function #8
   VTable: vtable_DSStepInto at 0191117c */

undefined4 __fastcall DSStepInto__vfunc_8(int *param_1)

{
  int iVar1;
  
  if (**(int **)(param_1[2] + 0x84) == 0) {
    FUN_00486000("Debugger->CallStack->StackDepth",".\\Debugger\\UnDebuggerCore.cpp",0x694);
  }
  if (*(int *)(param_1[2] + 0x58) != 0) {
    FUN_00486000("!Debugger->IsClosing",".\\Debugger\\UnDebuggerCore.cpp",0x695);
  }
  iVar1 = (**(code **)(*param_1 + 4))();
  if (iVar1 == 0) {
    FUN_00486000("Node",".\\Debugger\\UnDebuggerCore.cpp",0x697);
  }
  if (**(int **)(param_1[2] + 0x84) == param_1[4]) {
    if (*(int *)(iVar1 + 0xc) == 0) {
      FUN_00486000("Data","c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                   0x309);
    }
    if (*(int *)(*(int *)(iVar1 + 0xc) + -4 + *(int *)(iVar1 + 0x10) * 4) == param_1[3]) {
      return 0;
    }
  }
  return 1;
}




/* ========== DSStepOut.c ========== */

/*
 * SGW.exe - DSStepOut (1 functions)
 */

/* [VTable] DSStepOut virtual function #8
   VTable: vtable_DSStepOut at 01911150 */

undefined4 __fastcall DSStepOut__vfunc_8(int *param_1)

{
  int iVar1;
  undefined4 uVar2;
  
  if (**(int **)(param_1[2] + 0x84) == 0) {
    FUN_00486000("Debugger->CallStack->StackDepth",".\\Debugger\\UnDebuggerCore.cpp",0x67a);
  }
  if (*(int *)(param_1[2] + 0x58) != 0) {
    FUN_00486000("!Debugger->IsClosing",".\\Debugger\\UnDebuggerCore.cpp",0x67b);
  }
  iVar1 = (**(code **)(*param_1 + 4))();
  if (iVar1 == 0) {
    FUN_00486000("Node",".\\Debugger\\UnDebuggerCore.cpp",0x67e);
  }
  if (param_1[4] <= **(int **)(param_1[2] + 0x84)) {
    uVar2 = FDebuggerState__vfunc_8(param_1);
    return uVar2;
  }
  return 1;
}




/* ========== DSStepOverStack.c ========== */

/*
 * SGW.exe - DSStepOverStack (1 functions)
 */

/* [VTable] DSStepOverStack virtual function #8
   VTable: vtable_DSStepOverStack at 019111a8 */

uint __thiscall DSStepOverStack__vfunc_8(void *this,int param_1)

{
  int iVar1;
  uint uVar2;
  
  if (**(int **)(*(int *)((int)this + 8) + 0x84) == 0) {
    FUN_00486000("Debugger->CallStack->StackDepth",".\\Debugger\\UnDebuggerCore.cpp",0x69e);
  }
  if (*(int *)(*(int *)((int)this + 8) + 0x58) != 0) {
    FUN_00486000("!Debugger->IsClosing",".\\Debugger\\UnDebuggerCore.cpp",0x69f);
  }
  iVar1 = (**(code **)(*(int *)this + 4))();
  if (iVar1 == 0) {
    FUN_00486000("Node",".\\Debugger\\UnDebuggerCore.cpp",0x6a1);
  }
  if (**(int **)(*(int *)((int)this + 8) + 0x84) == *(int *)((int)this + 0x10)) {
    if (*(int *)(iVar1 + 0xc) == 0) {
      FUN_00486000("Data","c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\UnTemplate.h",
                   0x309);
    }
    if (*(int *)(*(int *)(iVar1 + 0xc) + -4 + *(int *)(iVar1 + 0x10) * 4) ==
        *(int *)((int)this + 0xc)) {
      return 0;
    }
  }
  iVar1 = **(int **)(*(int *)((int)this + 8) + 0x84);
  if (iVar1 < *(int *)((int)this + 0x10)) {
    return 1;
  }
  if (iVar1 == *(int *)((int)this + 0x10)) {
    return (uint)(param_1 == 0);
  }
  uVar2 = FDebuggerState__vfunc_8(this);
  return uVar2;
}




/* ========== DSWaitForInput.c ========== */

/*
 * SGW.exe - DSWaitForInput (4 functions)
 */

/* [VTable] DSWaitForInput virtual function #9
   VTable: vtable_DSWaitForInput at 019110c8 */

void __fastcall DSWaitForInput__vfunc_9(int *param_1)

{
  int iVar1;
  tagMSG local_1c;
  
  iVar1 = param_1[5];
  while (((iVar1 == 0 && (*(int *)(param_1[2] + 0x58) == 0)) && (DAT_01ead7ec == 0))) {
    if (DAT_01ee1254 != 0) {
      if (*(int *)(DAT_01ee1254 + 0x2cc) == 0) {
        FUN_00486000("GEngine->Client",".\\Debugger\\UnDebuggerCore.cpp",0x63e);
      }
      (**(code **)(**(int **)(DAT_01ee1254 + 0x2cc) + 0x114))(0);
    }
    iVar1 = PeekMessageW(&local_1c,(HWND)0x0,0,0,1);
    while (iVar1 != 0) {
      if (local_1c.message == 0x12) {
        DAT_01ead7ec = 1;
        (**(code **)(*param_1 + 0x28))();
      }
      TranslateMessage(&local_1c);
      DispatchMessageW(&local_1c);
      iVar1 = PeekMessageW(&local_1c,(HWND)0x0,0,0,1);
    }
    if (DAT_01ee1254 != 0) {
      (**(code **)(**(int **)(DAT_01ee1254 + 0x2cc) + 0x114))(1);
    }
    iVar1 = param_1[5];
  }
  return;
}


/* [VTable] DSWaitForInput virtual function #10
   VTable: vtable_DSWaitForInput at 019110c8 */

void __fastcall DSWaitForInput__vfunc_10(int param_1)

{
  *(undefined4 *)(param_1 + 0x14) = 1;
  return;
}


/* [VTable] DSWaitForInput virtual function #3
   VTable: vtable_DSWaitForInput at 019110c8 */

void __thiscall DSWaitForInput__vfunc_3(void *this,undefined4 param_1)

{
  int iVar1;
  undefined4 *puVar2;
  void *pvVar3;
  undefined4 *puVar4;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016d115f;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  (**(code **)(*(int *)this + 0x28))();
  switch(param_1) {
  case 1:
    DAT_01ead7ec = 1;
    FUN_009f50c0(*(void **)((int)this + 8));
    ExceptionList = pvStack_c;
    return;
  case 2:
    pvVar3 = (void *)FUN_00418e30(0x18);
    uStack_4 = 0;
    if (pvVar3 == (void *)0x0) {
LAB_009f6482:
      puVar4 = (undefined4 *)0x0;
    }
    else {
      puVar4 = FUN_009f31a0(pvVar3,*(int *)((int)this + 8));
    }
    break;
  default:
    goto switchD_009f6360_caseD_3;
  case 4:
    puVar4 = (undefined4 *)FUN_009f84c0(*(int *)(*(int *)((int)this + 8) + 0x84));
    if (puVar4 == (undefined4 *)0x0) {
      FUN_00486000("Top",".\\Debugger\\UnDebuggerCore.cpp",0x5db);
    }
    pvVar3 = (void *)FUN_00418e30(0x18);
    uStack_4 = 1;
    if (pvVar3 == (void *)0x0) goto LAB_009f6482;
    puVar4 = FUN_009f3200(pvVar3,*puVar4,*(int *)((int)this + 8));
    break;
  case 5:
    pvVar3 = (void *)FUN_00418e30(0x18);
    uStack_4 = 2;
    if (pvVar3 == (void *)0x0) goto LAB_009f6482;
    puVar4 = FUN_009f3140(pvVar3,*(int *)((int)this + 8));
    break;
  case 6:
    *(undefined4 *)(*(int *)((int)this + 8) + 0x54) = 0;
    ExceptionList = pvStack_c;
    return;
  case 7:
    pvVar3 = (void *)FUN_00418e30(0x14);
    uStack_4 = 3;
    if (pvVar3 == (void *)0x0) goto LAB_009f6482;
    puVar4 = FUN_009f2fc0(pvVar3,*(int *)((int)this + 8));
  }
  uStack_4 = 0xffffffff;
  iVar1 = *(int *)((int)this + 8);
  puVar2 = *(undefined4 **)(iVar1 + 0x7c);
  if (puVar2 != (undefined4 *)0x0) {
    (**(code **)*puVar2)(1);
  }
  *(undefined4 **)(iVar1 + 0x7c) = puVar4;
switchD_009f6360_caseD_3:
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] DSWaitForInput virtual function #2
   VTable: vtable_DSWaitForInput at 019110c8 */

void __fastcall DSWaitForInput__vfunc_2(int *param_1)

{
  if (*(int *)(param_1[2] + 0x58) == 0) {
    *(undefined4 *)(param_1[2] + 0x5c) = 0;
    *(undefined4 *)(param_1[2] + 100) = 0;
    FUN_009f6d70((void *)param_1[2]);
    param_1[5] = 0;
    (**(code **)(**(int **)(param_1[2] + 0x88) + 0xc))();
    (**(code **)(*param_1 + 0x24))();
  }
  return;
}




/* ========== DebugMessageCallback.c ========== */

/*
 * SGW.exe - DebugMessageCallback (1 functions)
 */

/* [VTable] DebugMessageCallback virtual function #1
   VTable: vtable_DebugMessageCallback at 018cae94 */

undefined4 * __thiscall DebugMessageCallback__vfunc_1(void *this,byte param_1)

{
  *(undefined ***)this = DebugMessageCallback::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== DefaultCriticalErrorHandler.c ========== */

/*
 * SGW.exe - DefaultCriticalErrorHandler (2 functions)
 */

/* [VTable] DefaultCriticalErrorHandler virtual function #1
   VTable: vtable_DefaultCriticalErrorHandler at 01922198 */

bool DefaultCriticalErrorHandler__vfunc_1(LPCSTR param_1)

{
  HWND hWnd;
  int iVar1;
  char *lpCaption;
  UINT uType;
  
  uType = 4;
  lpCaption = "Critical Error";
  hWnd = GetForegroundWindow();
  iVar1 = MessageBoxA(hWnd,param_1,lpCaption,uType);
  return iVar1 != 6;
}


/* Also in vtable: DefaultCriticalErrorHandler (slot 0) */

undefined4 * __thiscall DefaultCriticalErrorHandler__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_016d5c18;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = CriticalErrorHandler::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}




/* ========== FAudioEffect.c ========== */

/*
 * SGW.exe - FAudioEffect (1 functions)
 */

/* Also in vtable: FAudioEffect (slot 0) */

undefined4 * __thiscall FAudioEffect__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FAudioEffect::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FAudioEffectsManager.c ========== */

/*
 * SGW.exe - FAudioEffectsManager (1 functions)
 */

/* Also in vtable: FAudioEffectsManager (slot 0) */

undefined4 * __thiscall FAudioEffectsManager__vfunc_0(void *this,byte param_1)

{
  FUN_00958620(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FCallbackEventDevice.c ========== */

/*
 * SGW.exe - FCallbackEventDevice (1 functions)
 */

/* Also in vtable: FCallbackEventDevice (slot 0) */

undefined4 * __thiscall FCallbackEventDevice__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FCallbackEventDevice::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FCallbackEventDeviceEditor.c ========== */

/*
 * SGW.exe - FCallbackEventDeviceEditor (3 functions)
 */

/* Also in vtable: FCallbackEventDeviceEditor (slot 0) */

undefined4 * __thiscall FCallbackEventDeviceEditor__vfunc_0(void *this,byte param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_01677f08;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *(undefined ***)this = FCallbackEventDeviceEditor::vftable;
  local_4 = 0xffffffff;
  FUN_0041af70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = local_c;
  return this;
}


/* [VTable] FCallbackEventDeviceEditor virtual function #2
   VTable: vtable_FCallbackEventDeviceEditor at 017f91cc */

void __thiscall
FCallbackEventDeviceEditor__vfunc_2(void *this,int param_1,int param_2,undefined4 param_3)

{
  if (param_2 == 0) {
    FUN_00486000("InViewport",".\\Src\\FCallbackDeviceEditor.cpp",0x13);
  }
  if (param_1 == 0x20) {
    if ((*(int *)(DAT_01ef134c + 0x2dc) != 0) &&
       (*(int *)(*(int *)(DAT_01ef134c + 0x2dc) + 0x44) == param_2)) {
      *(undefined4 *)((int)this + 0x298) = DAT_01ee2684;
      FEdObjectPropagator__unknown_00b20ec0(*(undefined4 *)(DAT_01ef134c + 0x52c));
      FCallbackEventObserver__vfunc_2(this,0x20,param_2,param_3);
      return;
    }
    *(undefined4 *)((int)this + 0x298) = 0;
    FCallbackEventObserver__vfunc_2(this,0x20,param_2,param_3);
  }
  else {
    if (param_1 != 0x21) {
      FCallbackEventObserver__vfunc_2(this,param_1,param_2,param_3);
      return;
    }
    FCallbackEventObserver__vfunc_2(this,0x21,param_2,param_3);
    if (*(int *)((int)this + 0x298) != 0) {
      FEdObjectPropagator__unknown_00b20f10(*(int *)((int)this + 0x298));
      return;
    }
  }
  return;
}


/* [VTable] FCallbackEventDeviceEditor virtual function #4
   VTable: vtable_FCallbackEventDeviceEditor at 017f91cc */

void __thiscall FCallbackEventDeviceEditor__vfunc_4(void *this,int param_1,int param_2)

{
  int *piVar1;
  
  FCallbackEventObserver__vfunc_4(this,param_1,param_2);
  if ((param_2 != 0) && (piVar1 = *(int **)(param_2 + 0x20), piVar1 != (int *)0x0)) {
    if (param_1 == 0x1a) {
      (**(code **)(*piVar1 + 0x2c8))();
      return;
    }
    if (param_1 == 0x1b) {
      (**(code **)(*piVar1 + 0x2cc))();
    }
  }
  return;
}




/* ========== FCallbackEventObserver.c ========== */

/*
 * SGW.exe - FCallbackEventObserver (3 functions)
 */

/* [VTable] FCallbackEventObserver virtual function #4
   VTable: vtable_FCallbackEventObserver at 017f910c */

void __thiscall FCallbackEventObserver__vfunc_4(void *this,int param_1,undefined4 param_2)

{
  int iVar1;
  
  if (0x36 < param_1) {
    FUN_00486000("InType < CALLBACK_EventCount && \"Value is out of range\"",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\FCallbackDevice.h",0x14a
                );
  }
  iVar1 = 0;
  if (0 < *(int *)((int)this + param_1 * 0xc + 8)) {
    do {
      (**(code **)(**(int **)(*(int *)((int)this + param_1 * 0xc + 4) + iVar1 * 4) + 0x10))
                (param_1,param_2);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + param_1 * 0xc + 8));
  }
  return;
}


/* [VTable] FCallbackEventObserver virtual function #2
   VTable: vtable_FCallbackEventObserver at 017f910c */

void __thiscall
FCallbackEventObserver__vfunc_2(void *this,int param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  
  if (0x36 < param_1) {
    FUN_00486000("InType < CALLBACK_EventCount && \"Value is out of range\"",
                 "c:\\build\\qa\\sgw\\working\\development\\src\\core\\inc\\FCallbackDevice.h",0x16e
                );
  }
  iVar1 = 0;
  if (0 < *(int *)((int)this + param_1 * 0xc + 8)) {
    do {
      (**(code **)(**(int **)(*(int *)((int)this + param_1 * 0xc + 4) + iVar1 * 4) + 8))
                (param_1,param_2,param_3);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + param_1 * 0xc + 8));
  }
  return;
}


/* Also in vtable: FCallbackEventObserver (slot 0) */

undefined4 * __thiscall FCallbackEventObserver__vfunc_0(void *this,byte param_1)

{
  FUN_0041af70(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FCallbackQueryDevice.c ========== */

/*
 * SGW.exe - FCallbackQueryDevice (1 functions)
 */

/* Also in vtable: FCallbackQueryDevice (slot 0) */

undefined4 * __thiscall FCallbackQueryDevice__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = FCallbackQueryDevice::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== FCallbackQueryDeviceEditor.c ========== */

/*
 * SGW.exe - FCallbackQueryDeviceEditor (3 functions)
 */

/* Also in vtable: FCallbackQueryDeviceEditor (slot 0) */

undefined4 * __thiscall FCallbackQueryDeviceEditor__vfunc_0(void *this,byte param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_01677a08;
  local_c = ExceptionList;
  local_4 = 0xffffffff;
  ExceptionList = &local_c;
  *(undefined ***)this = FCallbackQueryDevice::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* [VTable] FCallbackQueryDeviceEditor virtual function #1
   VTable: vtable_FCallbackQueryDeviceEditor at 017f8dd8 */

undefined4 FCallbackQueryDeviceEditor__vfunc_1(int param_1,int param_2)

{
  int iVar1;
  undefined4 uVar2;
  
  if (param_1 == 1) {
    iVar1 = FUN_004adfc0(param_2);
    if ((DAT_01ef2e70 != (int *)0x0) && (iVar1 != 0)) {
      uVar2 = (**(code **)(*DAT_01ef2e70 + 0x39c))(iVar1);
      return uVar2;
    }
  }
  else {
    FUN_00486000("0",".\\Src\\FCallbackDeviceEditor.cpp",0x74);
  }
  return 0;
}


/* [VTable] FCallbackQueryDeviceEditor virtual function #2
   VTable: vtable_FCallbackQueryDeviceEditor at 017f8dd8 */

undefined4 FCallbackQueryDeviceEditor__vfunc_2(int param_1,undefined4 *param_2)

{
  bool bVar1;
  bool bVar2;
  bool bVar3;
  bool bVar4;
  void *pvVar5;
  undefined4 *puVar6;
  undefined4 *puVar7;
  undefined **_Str1;
  int iVar8;
  undefined **_Str2;
  undefined4 uVar9;
  int local_3c [3];
  int local_30 [3];
  int local_24 [3];
  int local_18 [3];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_017148a4;
  local_c = ExceptionList;
  bVar4 = false;
  bVar3 = false;
  bVar2 = false;
  bVar1 = false;
  if (param_1 != 0) {
    ExceptionList = &local_c;
    FUN_00486000("0",".\\Src\\FCallbackDeviceEditor.cpp",0x5e);
    ExceptionList = local_c;
    return 0;
  }
  ExceptionList = &local_c;
  if (DAT_01ef134c != 0) {
    ExceptionList = &local_c;
    pvVar5 = FMapPackageFileCache__unknown_004d39f0(local_18,(undefined4 *)(DAT_01ef134c + 0x514));
    local_4 = 0;
    puVar6 = FUN_00488b90(pvVar5,local_24,1);
    local_4 = 1;
    pvVar5 = FMapPackageFileCache__unknown_004d39f0(local_30,param_2);
    local_4 = 2;
    puVar7 = FUN_00488b90(pvVar5,local_3c,1);
    local_4 = 3;
    bVar4 = true;
    bVar3 = true;
    bVar2 = true;
    bVar1 = true;
    if (puVar6[1] == 0) {
      _Str2 = &PTR_017f8e10;
    }
    else {
      _Str2 = (undefined **)*puVar6;
    }
    if (puVar7[1] == 0) {
      _Str1 = &PTR_017f8e10;
    }
    else {
      _Str1 = (undefined **)*puVar7;
    }
    iVar8 = _wcsicmp((wchar_t *)_Str1,(wchar_t *)_Str2);
    if (iVar8 == 0) {
      uVar9 = 1;
      goto LAB_00ecfa68;
    }
  }
  uVar9 = 0;
LAB_00ecfa68:
  local_4 = 2;
  if (bVar2) {
    FUN_0041b420(local_3c);
  }
  if (bVar3) {
    local_4 = 1;
    FUN_0041b420(local_30);
  }
  local_4 = 0;
  if (bVar4) {
    FUN_0041b420(local_24);
  }
  if (bVar1) {
    local_4 = 0xffffffff;
    FUN_0041b420(local_18);
  }
  ExceptionList = local_c;
  return uVar9;
}




/* ========== UBodyComponent.c ========== */

/*
 * SGW.exe - UBodyComponent (5 functions)
 */

/* Also in vtable: UBodyComponent (slot 0) */

undefined4 UBodyComponent__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBodyComponent virtual function #1
   VTable: vtable_UBodyComponent at 01968c7c */

bool __fastcall UBodyComponent__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UBodyComponent virtual function #2
   VTable: vtable_UBodyComponent at 01968c7c */

int * __thiscall UBodyComponent__vfunc_2(void *this,byte param_1)

{
  FUN_00b29280(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UBodyComponent virtual function #19
   VTable: vtable_UBodyComponent at 01968c7c */

void __fastcall UBodyComponent__vfunc_19(undefined4 param_1)

{
  undefined4 uStack00000004;
  
  UTestIpDrv__vfunc_19(param_1);
  uStack00000004 = 0x18;
                    /* WARNING: Could not recover jumptable at 0x0156d8bd. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*DAT_01ea577c + 0x1c))();
  return;
}


/* [VTable] UBodyComponent virtual function #7
   VTable: vtable_UBodyComponent at 01968c7c */

void __fastcall UBodyComponent__vfunc_7(void *param_1)

{
  bool bVar1;
  int *piVar2;
  int iVar3;
  int iVar4;
  int *piVar5;
  
  UTestIpDrv__vfunc_7(param_1);
  if (DAT_01ead7ac != 0) {
    iVar4 = 0;
    bVar1 = false;
    if (0 < *(int *)((int)param_1 + 0x44)) {
      iVar3 = 0;
      do {
        piVar5 = (int *)(*(int *)((int)param_1 + 0x40) + iVar3);
        if ((*piVar5 != 0) && (piVar5[2] == 0)) {
          piVar2 = (int *)FUN_0156d950(4,piVar5 + 1);
          if (piVar2 != (int *)0x0) {
            *piVar2 = *piVar5;
          }
          *piVar5 = 0;
          bVar1 = true;
        }
        iVar4 = iVar4 + 1;
        iVar3 = iVar3 + 0x5c;
      } while (iVar4 < *(int *)((int)param_1 + 0x44));
      if (bVar1) {
        FUN_004a0410(param_1,1);
      }
    }
  }
  return;
}




/* ========== UBodyComponentFactory.c ========== */

/*
 * SGW.exe - UBodyComponentFactory (3 functions)
 */

/* Also in vtable: UBodyComponentFactory (slot 0) */

undefined4 UBodyComponentFactory__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBodyComponentFactory virtual function #1
   VTable: vtable_UBodyComponentFactory at 0196891c */

bool __fastcall UBodyComponentFactory__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee11dc == (undefined4 *)0x0) {
    DAT_01ee11dc = FUN_00500b90();
    FUN_00500940();
  }
  return puVar1 != DAT_01ee11dc;
}


/* [VTable] UBodyComponentFactory virtual function #2
   VTable: vtable_UBodyComponentFactory at 0196891c */

int * __thiscall UBodyComponentFactory__vfunc_2(void *this,byte param_1)

{
  FUN_00b28430(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UBodySet.c ========== */

/*
 * SGW.exe - UBodySet (4 functions)
 */

/* Also in vtable: UBodySet (slot 0) */

undefined4 UBodySet__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBodySet virtual function #1
   VTable: vtable_UBodySet at 01968b6c */

bool __fastcall UBodySet__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UBodySet virtual function #2
   VTable: vtable_UBodySet at 01968b6c */

int * __thiscall UBodySet__vfunc_2(void *this,byte param_1)

{
  FUN_00b28cc0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UBodySet virtual function #19
   VTable: vtable_UBodySet at 01968b6c */

void __fastcall UBodySet__vfunc_19(undefined4 param_1)

{
  undefined4 uStack00000004;
  
  UTestIpDrv__vfunc_19(param_1);
  uStack00000004 = 0x18;
                    /* WARNING: Could not recover jumptable at 0x0156d8dd. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*DAT_01ea577c + 0x1c))();
  return;
}




/* ========== UBodySetFactory.c ========== */

/*
 * SGW.exe - UBodySetFactory (3 functions)
 */

/* Also in vtable: UBodySetFactory (slot 0) */

undefined4 UBodySetFactory__vfunc_0(void)

{
  return 1;
}


/* [VTable] UBodySetFactory virtual function #1
   VTable: vtable_UBodySetFactory at 01968a44 */

bool __fastcall UBodySetFactory__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee11dc == (undefined4 *)0x0) {
    DAT_01ee11dc = FUN_00500b90();
    FUN_00500940();
  }
  return puVar1 != DAT_01ee11dc;
}


/* [VTable] UBodySetFactory virtual function #2
   VTable: vtable_UBodySetFactory at 01968a44 */

int * __thiscall UBodySetFactory__vfunc_2(void *this,byte param_1)

{
  FUN_00b284a0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UBodySlotSelector_CustomInputProxy.c ========== */

/*
 * SGW.exe - UBodySlotSelector_CustomInputProxy (1 functions)
 */

undefined4 UBodySlotSelector_CustomInputProxy__vfunc_0(void)

{
  return 1;
}




/* ========== UConsole.c ========== */

/*
 * SGW.exe - UConsole (3 functions)
 */

/* Also in vtable: UConsole (slot 0) */

undefined4 UConsole__vfunc_0(void)

{
  return 0;
}


/* [VTable] UConsole virtual function #1
   VTable: vtable_UConsole at 0186581c */

bool __fastcall UConsole__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee30c8 == (undefined4 *)0x0) {
    DAT_01ee30c8 = FUN_00681690();
    FUN_006807b0();
  }
  return puVar1 != DAT_01ee30c8;
}


/* [VTable] UConsole virtual function #2
   VTable: vtable_UConsole at 0186581c */

int * __thiscall UConsole__vfunc_2(void *this,byte param_1)

{
  FUN_00682f70(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UConsoleEntry.c ========== */

/*
 * SGW.exe - UConsoleEntry (4 functions)
 */

/* Also in vtable: UConsoleEntry (slot 0) */

undefined4 UConsoleEntry__vfunc_0(void)

{
  return 1;
}


/* [VTable] UConsoleEntry virtual function #138
   VTable: vtable_UConsoleEntry at 0185943c */

void __thiscall UConsoleEntry__vfunc_138(void *this,float *param_1)

{
  int *piVar1;
  short sVar2;
  int iVar3;
  int iVar4;
  undefined4 *puVar5;
  short *psVar6;
  int iVar7;
  short *psVar8;
  undefined **ppuVar9;
  int iVar10;
  int iVar11;
  int iVar12;
  int *piVar13;
  float local_64;
  int local_5c [6];
  undefined4 uStack_44;
  undefined4 uStack_40;
  float fStack_3c;
  undefined4 uStack_38;
  undefined4 uStack_34;
  undefined4 uStack_30;
  int *piStack_2c;
  undefined1 uStack_28;
  undefined1 uStack_27;
  undefined4 uStack_24;
  undefined4 uStack_20;
  undefined4 uStack_1c;
  undefined4 uStack_18;
  undefined4 uStack_14;
  undefined4 uStack_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_01697ac1;
  local_c = ExceptionList;
  if (*(int *)((int)this + 0x328) == 0) {
    if (*(int *)((int)this + 0x32c) == 0) {
      return;
    }
    iVar4 = *(int *)((int)this + 0x32c);
    iVar10 = *(int *)(iVar4 + 0x358);
  }
  else {
    iVar4 = *(int *)((int)this + 0x328);
    iVar10 = *(int *)(iVar4 + 0x35c);
  }
  if ((iVar4 != 0) && ((*(byte *)((int)this + 0x334) & 1) != 0)) {
    ExceptionList = &local_c;
    if (DAT_01dc3900 == -1) {
      ExceptionList = &local_c;
      puVar5 = FUN_0041aab0(local_5c + 3,(short *)&DAT_01856f20);
      if (puVar5[1] == 0) {
        DAT_01dc3900 = 0;
      }
      else {
        DAT_01dc3900 = puVar5[1] + -1;
      }
      local_4 = 0xffffffff;
      FUN_0041b420(local_5c + 3);
    }
    iVar3 = *(int *)(*(int *)(iVar10 + 0x80) + 0x40);
    iVar11 = *(int *)((int)this + 0x330) + DAT_01dc3900;
    piVar13 = (int *)(*(int *)(iVar10 + 0x80) + 0x3c);
    iVar12 = 0;
    local_64 = 0.0;
    if (iVar3 != 1 && -1 < iVar3 + -1) {
      do {
        piVar1 = (int *)(*piVar13 + iVar12 * 4);
        psVar6 = (short *)(**(code **)(*(int *)*piVar1 + 0x10))(1);
        psVar8 = psVar6 + 1;
        do {
          sVar2 = *psVar6;
          psVar6 = psVar6 + 1;
        } while (sVar2 != 0);
        iVar7 = (int)psVar6 - (int)psVar8 >> 1;
        if (iVar11 < iVar7) break;
        local_64 = *(float *)(*piVar1 + 0x1c) + local_64;
        iVar11 = iVar11 - iVar7;
        iVar12 = iVar12 + 1;
      } while (iVar12 < iVar3 + -1);
    }
    local_5c[0] = 0;
    local_5c[1] = 0;
    local_5c[2] = 0;
    local_4 = 3;
    if (0 < iVar3) {
      psVar8 = (short *)(**(code **)(**(int **)(*piVar13 + iVar12 * 4) + 0x10))(1);
      FUN_005543f0(local_5c,psVar8);
    }
    uStack_44 = 0;
    uStack_40 = 0;
    fStack_3c = 0.0;
    uStack_38 = 0;
    uStack_34 = DAT_017f7ea0;
    uStack_30 = DAT_017f7ea0;
    piStack_2c = (int *)0x0;
    uStack_24 = 0;
    uStack_20 = 0;
    uStack_1c = 0;
    uStack_18 = 0;
    uStack_14 = 0;
    uStack_10 = 0;
    uStack_28 = 3;
    uStack_27 = 3;
    if ((*(byte *)(*(int *)(iVar10 + 0x80) + 0xa8) & 1) == 0) {
      FUN_00486000("StringRenderComponent->ValueString->StringStyleData.IsInitialized()",
                   ".\\Src\\UnUIObjects.cpp",0x2bc9);
    }
    if (*(int *)(*(int *)(iVar10 + 0x80) + 0x68) == 0) {
      FUN_00486000("StringRenderComponent->ValueString->StringStyleData.DrawFont",
                   ".\\Src\\UnUIObjects.cpp",0x2bca);
    }
    piStack_2c = *(int **)(*(int *)(iVar10 + 0x80) + 0x68);
    puVar5 = FUN_00425e70(local_5c,local_5c + 3,iVar11);
    local_4._0_1_ = 4;
    if (puVar5[1] == 0) {
      ppuVar9 = &PTR_017f8e10;
    }
    else {
      ppuVar9 = (undefined **)*puVar5;
    }
    FUN_0085a660((int)&uStack_44,(short *)ppuVar9,(short *)0x0);
    local_4 = CONCAT31(local_4._1_3_,3);
    FUN_0041b420(local_5c + 3);
    USequenceOp__unknown_005f92c0
              (param_1,*(float *)(iVar4 + 0x1f0) + fStack_3c,
               *(float *)(iVar4 + 500) + *(float *)(iVar4 + 0x1dc) + local_64,
               (ushort *)&DAT_01856fdc,piStack_2c,(undefined8 *)(*(int *)(iVar10 + 0x80) + 0x48),1.0
               ,1.0);
    local_4 = 0xffffffff;
    FUN_0041b420(local_5c);
  }
  ExceptionList = local_c;
  return;
}


/* [VTable] UConsoleEntry virtual function #1
   VTable: vtable_UConsoleEntry at 0185943c */

bool __fastcall UConsoleEntry__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2f18 == (undefined4 *)0x0) {
    DAT_01ee2f18 = FUN_006459c0();
    FUN_006477c0();
  }
  return puVar1 != DAT_01ee2f18;
}


/* [VTable] UConsoleEntry virtual function #2
   VTable: vtable_UConsoleEntry at 0185943c */

int * __thiscall UConsoleEntry__vfunc_2(void *this,byte param_1)

{
  FUN_00656da0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UDebugManager.c ========== */

/*
 * SGW.exe - UDebugManager (3 functions)
 */

/* Also in vtable: UDebugManager (slot 0) */

undefined4 UDebugManager__vfunc_0(void)

{
  return 1;
}


/* [VTable] UDebugManager virtual function #1
   VTable: vtable_UDebugManager at 01834d8c */

bool __fastcall UDebugManager__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] UDebugManager virtual function #2
   VTable: vtable_UDebugManager at 01834d8c */

int * __thiscall UDebugManager__vfunc_2(void *this,byte param_1)

{
  FUN_00533af0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UDebugger.c ========== */

/*
 * SGW.exe - UDebugger (1 functions)
 */

/* Also in vtable: UDebugger (slot 0) */

undefined4 * __thiscall UDebugger__vfunc_0(void *this,byte param_1)

{
  *(undefined ***)this = UDebugger::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== UDebuggerCore.c ========== */

/*
 * SGW.exe - UDebuggerCore (6 functions)
 */

/* [VTable] UDebuggerCore virtual function #2
   VTable: vtable_UDebuggerCore at 01911238 */

void __fastcall UDebuggerCore__vfunc_2(int param_1)

{
  *(undefined4 *)(param_1 + 0x50) = 1;
  return;
}


/* [VTable] UDebuggerCore virtual function #4
   VTable: vtable_UDebuggerCore at 01911238 */

void __fastcall UDebuggerCore__vfunc_4(int param_1)

{
  *(undefined4 *)(param_1 + 0x5c) = 1;
  return;
}


/* [VTable] UDebuggerCore virtual function #3
   VTable: vtable_UDebuggerCore at 01911238 */

void UDebuggerCore__vfunc_3(void)

{
  return;
}


/* [VTable] UDebuggerCore virtual function #5
   VTable: vtable_UDebuggerCore at 01911238 */

undefined4 __fastcall UDebuggerCore__vfunc_5(void *param_1)

{
  void *this;
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d0ca8;
  local_c = ExceptionList;
  if ((DAT_01ead7ec == 0) && (*(int *)((int)param_1 + 0x58) == 0)) {
    ExceptionList = &local_c;
    this = (void *)scalable_malloc(0x18);
    if (this == (void *)0x0) {
      FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_4 = 0;
    puVar1 = FUN_009f3020(this,(int)param_1);
    local_4 = 0xffffffff;
    FUN_009f3320(param_1,(int)puVar1,1);
    if ((DAT_01ead7ec == 0) && (*(int *)((int)param_1 + 0x58) == 0)) {
      ExceptionList = local_c;
      return 1;
    }
  }
  ExceptionList = local_c;
  return 0;
}


/* [VTable] UDebuggerCore virtual function #6
   VTable: vtable_UDebuggerCore at 01911238 */

undefined4 __fastcall UDebuggerCore__vfunc_6(void *param_1)

{
  void *this;
  undefined4 *puVar1;
  exception local_18 [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016d0cbd;
  local_c = ExceptionList;
  if ((DAT_01ead7ec == 0) && (*(int *)((int)param_1 + 0x58) == 0)) {
    ExceptionList = &local_c;
    this = (void *)scalable_malloc(0x18);
    if (this == (void *)0x0) {
      FUN_004099f0(local_18);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_18,(ThrowInfo *)&DAT_01d65cc8);
    }
    local_4 = 0;
    puVar1 = FUN_009f3020(this,(int)param_1);
    local_4 = 0xffffffff;
    FUN_009f3320(param_1,(int)puVar1,1);
    if ((DAT_01ead7ec == 0) && (*(int *)((int)param_1 + 0x58) == 0)) {
      ExceptionList = local_c;
      return 1;
    }
  }
  ExceptionList = local_c;
  return 0;
}


/* Also in vtable: UDebuggerCore (slot 0) */

undefined4 * __thiscall UDebuggerCore__vfunc_0(void *this,byte param_1)

{
  FUN_009f7e50(this);
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  return this;
}




/* ========== UEditorUISceneClient.c ========== */

/*
 * SGW.exe - UEditorUISceneClient (1 functions)
 */

undefined4 UEditorUISceneClient__vfunc_0(void)

{
  return 1;
}




/* ========== UGameUISceneClient.c ========== */

/*
 * SGW.exe - UGameUISceneClient (28 functions)
 */

/* Also in vtable: UGameUISceneClient (slot 0) */

undefined4 UGameUISceneClient__vfunc_0(void)

{
  return 0;
}


/* [VTable] UGameUISceneClient virtual function #74
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_74(void *this,int param_1,float *param_2)

{
  int iVar1;
  
  iVar1 = UUISceneClient__vfunc_74(this,param_1,param_2);
  if ((((iVar1 == 1) && (param_1 != 0)) && (iVar1 = *(int *)(param_1 + 0x164), iVar1 != 0)) &&
     (*(char *)(param_1 + 0x1a1) == '\x01')) {
    *param_2 = *(float *)(iVar1 + 0x70) * *param_2;
    param_2[1] = *(float *)(iVar1 + 0x74) * param_2[1];
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #75
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_75(int *param_1)

{
  int iVar1;
  float *pfVar2;
  float unaff_ESI;
  float unaff_EDI;
  float *pfVar3;
  float local_58 [21];
  
  iVar1 = (**(code **)(*param_1 + 0x128))(0,local_58);
  if (iVar1 == 0) {
    unaff_EDI = DAT_01862130;
    unaff_ESI = DAT_0186212c;
  }
  pfVar2 = FUN_005f7290(local_58,(int)unaff_EDI,(int)unaff_ESI,DAT_01836f34,DAT_01daea80);
  pfVar3 = (float *)(param_1 + 0x1c);
  for (iVar1 = 0x10; iVar1 != 0; iVar1 = iVar1 + -1) {
    *pfVar3 = *pfVar2;
    pfVar2 = pfVar2 + 1;
    pfVar3 = pfVar3 + 1;
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #68
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_68(void *this,undefined4 param_1,undefined4 param_2)

{
  if (*(int *)((int)this + 0x44) != 0) {
    (**(code **)(**(int **)((int)this + 0x44) + 0x34))(param_1,param_2);
  }
  (**(code **)(*(int *)this + 0x168))();
  return;
}


/* [VTable] UGameUISceneClient virtual function #90
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_90(int param_1)

{
  if (*(int *)(param_1 + 0x44) != 0) {
    (**(code **)(**(int **)(param_1 + 0x44) + 0x30))(param_1 + 0x4c);
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #69
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall UGameUISceneClient__vfunc_69(void *this,uint param_1,int param_2)

{
  int iVar1;
  
  if (*(void **)((int)this + 0x48) != (void *)0x0) {
    if ((param_1 != 0) || (param_2 != 0)) {
      iVar1 = FUN_008626b0(*(void **)((int)this + 0x48),param_1,param_2);
      if (iVar1 != 0) {
        *(int *)((int)this + 0xc4) = iVar1;
        return 1;
      }
    }
  }
  return 0;
}


/* [VTable] UGameUISceneClient virtual function #96
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_96(int *param_1)

{
  int iVar1;
  undefined4 uVar2;
  undefined1 *puVar3;
  undefined4 uVar4;
  
  (**(code **)(*param_1 + 0x15c))(5);
  iVar1 = *param_1;
  uVar4 = 0;
  puVar3 = &stack0xfffffff8;
  uVar2 = FUN_0049ffb0(param_1,DAT_01ee13cc,DAT_01ee13d0,0);
  (**(code **)(iVar1 + 0xe8))(uVar2,puVar3,uVar4);
  return;
}


/* [VTable] UGameUISceneClient virtual function #72
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_72(int param_1)

{
  int *piVar1;
  int in_stack_ffffffec;
  int in_stack_fffffff0;
  int iVar2;
  
  iVar2 = 0;
  FUN_0049e960(&stack0xffffffec,L"Styles",1);
  piVar1 = (int *)UDataStoreClient__unknown_0065fb30
                            (*(void **)(param_1 + 0x58),in_stack_ffffffec,in_stack_fffffff0,iVar2);
  if (piVar1 != (int *)0x0) {
    FUN_00660d90(*(void **)(param_1 + 0x58),piVar1);
  }
  iVar2 = FUN_0065ffc0(*(void **)(param_1 + 0x58),*(int **)(param_1 + 0x48));
  if (iVar2 != 0) {
    FUN_00863b80(*(int **)(param_1 + 0x48));
    iVar2 = 0;
    if (0 < *(int *)(param_1 + 0xbc)) {
      do {
        piVar1 = *(int **)(*(int *)(param_1 + 0xb8) + iVar2 * 4);
        if ((piVar1 != (int *)0x0) && ((*(byte *)(piVar1 + 0x17) & 4) != 0)) {
          (**(code **)(*piVar1 + 0x19c))();
        }
        iVar2 = iVar2 + 1;
      } while (iVar2 < *(int *)(param_1 + 0xbc));
    }
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #71
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_71(void *this,int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)((int)this + 0x44);
  *(int *)((int)this + 0x44) = param_1;
  (**(code **)(*(int *)((int)this + 0x40) + 8))(0x2d,param_1,0);
  if ((iVar1 != param_1) && (iVar1 = 0, 0 < *(int *)((int)this + 0xbc))) {
    do {
      (**(code **)(**(int **)(*(int *)((int)this + 0xb8) + iVar1 * 4) + 0x180))(0,1,0,0);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + 0xbc));
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #73
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall UGameUISceneClient__vfunc_73(void *this,int param_1,float *param_2)

{
  float fVar1;
  int iVar2;
  float local_8;
  float local_4;
  
  *param_2 = 0.0;
  param_2[1] = 0.0;
  if (((param_1 != 0) && (*(int *)(param_1 + 0x164) != 0)) && (*(char *)(param_1 + 0x1a1) == '\x01')
     ) {
    local_8 = 0.0;
    local_4 = 0.0;
    iVar2 = UUISceneClient__vfunc_74(this,param_1,&local_8);
    if (iVar2 != 0) {
      fVar1 = *(float *)(*(int *)(param_1 + 0x164) + 0x6c);
      *param_2 = *param_2 + *(float *)(*(int *)(param_1 + 0x164) + 0x68) * local_8;
      param_2[1] = fVar1 * local_4 + param_2[1];
    }
  }
  return 1;
}


/* [VTable] UGameUISceneClient virtual function #95
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_95(int *param_1)

{
  int iVar1;
  int *piVar2;
  uint uVar3;
  undefined1 auStack_14 [8];
  
  iVar1 = 0;
  uVar3 = 0;
  if (0 < param_1[0x2f]) {
    piVar2 = (int *)param_1[0x2e];
    do {
      if ((*(uint *)(*piVar2 + 0x198) & 0x100) != 0) {
        uVar3 = 1;
        break;
      }
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 1;
    } while (iVar1 < param_1[0x2f]);
  }
  iVar1 = *(int *)(param_1[10] + 0x28);
  *(uint *)(iVar1 + 0xcc) = *(uint *)(iVar1 + 0xcc) ^ (*(uint *)(iVar1 + 0xcc) ^ uVar3) & 1;
  param_1[0x32] = param_1[0x32] ^ (*(uint *)(iVar1 + 0xcc) ^ param_1[0x32]) & 1;
  if (((*(byte *)(param_1 + 0x32) & 1) != 0) && (param_1[0x31] == 0)) {
    iVar1 = *param_1;
    FUN_0049e960(auStack_14,L"Arrow",1);
    (**(code **)(iVar1 + 0x114))();
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #89
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_89(void *this,int param_1)

{
  int iVar1;
  
  if (-1 < param_1) {
    iVar1 = *(int *)(*(int *)((int)this + 0x28) + 0x28);
    *(uint *)(iVar1 + 0xcc) =
         *(uint *)(iVar1 + 0xcc) ^ ((uint)(0 < param_1) ^ *(uint *)(iVar1 + 0xcc)) & 1;
    *(uint *)((int)this + 200) =
         *(uint *)((int)this + 200) ^ (*(uint *)(iVar1 + 0xcc) ^ *(uint *)((int)this + 200)) & 1;
    return;
  }
  (**(code **)(*(int *)this + 0x17c))();
  return;
}


/* [VTable] UGameUISceneClient virtual function #83
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_83(int param_1)

{
  uint *puVar1;
  
  puVar1 = (uint *)(*(int *)(param_1 + 0x28) + 0x108);
  *puVar1 = *puVar1 & 0xfffffffe;
  return;
}


/* [VTable] UGameUISceneClient virtual function #84
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_84(void *this,int param_1)

{
  int iVar1;
  
  if (param_1 == 0) {
    FUN_00486000("CanvasScene",".\\Src\\UnUserInterface.cpp",0x5d1);
  }
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0xbc)) {
    do {
      (**(code **)(**(int **)(*(int *)((int)this + 0xb8) + iVar1 * 4) + 0x1ec))(param_1);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + 0xbc));
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #78
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall UGameUISceneClient__vfunc_78(int param_1,int param_2,undefined4 param_3)

{
  undefined4 uVar1;
  int iVar2;
  int iVar3;
  
  uVar1 = 0;
  iVar3 = *(int *)(param_1 + 0xbc) + -1;
  if (-1 < iVar3) {
    while (iVar2 = FUN_0063c7f0(*(void **)(*(int *)(param_1 + 0xb8) + iVar3 * 4),param_2,param_3),
          iVar2 == 0) {
      iVar3 = iVar3 + -1;
      if (iVar3 < 0) {
        return 0;
      }
    }
    uVar1 = 1;
  }
  return uVar1;
}


/* [VTable] UGameUISceneClient virtual function #80
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall
UGameUISceneClient__vfunc_80(void *this,int param_1,int param_2,undefined4 *param_3)

{
  int iVar1;
  undefined4 *puVar2;
  int iVar3;
  undefined4 unaff_EBP;
  undefined4 unaff_EDI;
  
  puVar2 = param_3;
  iVar1 = param_2;
  iVar3 = param_1;
  if (param_1 != 0) {
    if (param_3 != (undefined4 *)0x0) {
      *param_3 = 0;
    }
    param_1 = UGameUISceneClient__unknown_00677280
                        (this,*(int *)(param_1 + 0x154),*(int *)(param_1 + 0x158),param_2);
    if ((param_1 == 0) &&
       (iVar3 = (**(code **)(*(int *)this + 0x13c))(iVar3,iVar1,&param_1), iVar3 != 0)) {
      (**(code **)(*(int *)this + 0x174))(unaff_EDI);
      if (puVar2 != (undefined4 *)0x0) {
        *puVar2 = unaff_EBP;
      }
      return 1;
    }
  }
  return 0;
}


/* [VTable] UGameUISceneClient virtual function #67
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall UGameUISceneClient__vfunc_67(void *this,int param_1)

{
  int iVar1;
  
  if (param_1 != 0) {
    iVar1 = UGameUISceneClient__unknown_00677280
                      (this,*(int *)(param_1 + 0x154),*(int *)(param_1 + 0x158),
                       *(int *)(param_1 + 0x164));
    if (iVar1 != 0) {
      (**(code **)(*(int *)this + 0x178))(iVar1);
      return 1;
    }
  }
  return 0;
}


/* [VTable] UGameUISceneClient virtual function #87
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall UGameUISceneClient__vfunc_87(void *this,byte param_1)

{
  int *piVar1;
  undefined4 uVar2;
  int *piVar3;
  int iVar4;
  undefined4 uStack_4;
  
  uVar2 = 0;
  iVar4 = 0;
  uStack_4 = this;
  if (0 < *(int *)((int)this + 0xbc)) {
    while (piVar1 = *(int **)(*(int *)((int)this + 0xb8) + iVar4 * 4), piVar1 == (int *)0x0) {
LAB_00679268:
      iVar4 = iVar4 + 1;
      if (*(int *)((int)this + 0xbc) <= iVar4) {
        return 0;
      }
    }
    if ((param_1 & 1) == 0) {
      piVar3 = (int *)UGameUISceneClient__unknown_00677280(this,0x6b,0,0);
      if (piVar3 == (int *)0x0) {
        FUN_00486000("TransientScene",".\\Src\\UnUserInterface.cpp",0x4b7);
      }
      if (piVar1 != piVar3) goto LAB_00679210;
    }
    else {
LAB_00679210:
      if ((param_1 & 2) != 0) {
        uStack_4 = (void *)((uint)uStack_4 & 0xffffff);
        (**(code **)(*piVar1 + 0xec))(DAT_01ee1bf4,DAT_01ee1bf8,piVar1 + 0x70,(int)&uStack_4 + 3,0);
        if (uStack_4._3_1_ != '\0') goto LAB_00679281;
      }
      if ((((param_1 & 4) == 0) || ((piVar1[0x66] & 0x800U) == 0)) &&
         (((param_1 & 8) == 0 || ((*(byte *)(piVar1 + 0x66) & 0x80) == 0)))) goto LAB_00679268;
    }
LAB_00679281:
    uVar2 = 1;
  }
  return uVar2;
}


/* [VTable] UGameUISceneClient virtual function #1
   VTable: vtable_UGameUISceneClient at 01864954 */

bool __fastcall UGameUISceneClient__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee307c == (undefined4 *)0x0) {
    DAT_01ee307c = FUN_00678580();
    FUN_00678650();
  }
  return puVar1 != DAT_01ee307c;
}


/* [VTable] UGameUISceneClient virtual function #93
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_93(void *this,int *param_1)

{
  code *pcVar1;
  undefined4 *puVar2;
  int iVar3;
  int iVar4;
  int iVar5;
  int iVar6;
  
  puVar2 = (undefined4 *)FUN_0067e4d0(4,(int *)((int)this + 0xb8));
  if (puVar2 != (undefined4 *)0x0) {
    *puVar2 = param_1;
  }
  iVar5 = 0;
  if (param_1[0x59] != 0) {
    iVar5 = FUN_00681850(param_1[0x59]);
  }
  param_1[0x65] = iVar5;
  FUN_006438c0(param_1);
  if ((*(byte *)(param_1 + 0x66) & 0x80) != 0) {
    (**(code **)(*(int *)this + 0x14c))(param_1);
  }
  iVar3 = UUIScreenObject__unknown_00633e40(param_1);
  if (iVar3 < 1) {
    iVar3 = (**(code **)(*param_1 + 0x148))(iVar5);
    if (iVar3 != 0) {
      (**(code **)(*param_1 + 0x14c))(0,iVar5);
    }
  }
  else {
    iVar6 = 0;
    if (0 < iVar3) {
      do {
        pcVar1 = *(code **)(*param_1 + 0x148);
        param_1[0x65] = iVar6;
        iVar4 = (*pcVar1)(iVar6);
        if (iVar4 != 0) {
          (**(code **)(*param_1 + 0x14c))(0,iVar6);
        }
        iVar6 = iVar6 + 1;
      } while (iVar6 < iVar3);
    }
    param_1[0x65] = iVar5;
  }
  (**(code **)(*(int *)this + 0x16c))();
  (**(code **)(*param_1 + 0x218))(1);
  return;
}


/* [VTable] UGameUISceneClient virtual function #85
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_85(void *this,undefined4 param_1)

{
  int iVar1;
  int iVar2;
  int local_18;
  int local_14;
  undefined4 local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_0169b3ca;
  local_c = ExceptionList;
  iVar2 = 0;
  local_18 = 0;
  local_14 = 0;
  local_10 = 0;
  local_4 = 1;
  ExceptionList = &local_c;
  FUN_0067a990(this,&local_18);
  if (0 < local_14) {
    do {
      iVar1 = *(int *)(local_18 + iVar2 * 4);
      if ((~*(uint *)(iVar1 + 0x5c) & 1) != 0) {
        (**(code **)(*(int *)this + 0x158))(param_1,iVar1);
      }
      iVar2 = iVar2 + 1;
    } while (iVar2 < local_14);
  }
  local_4 = 0xffffffff;
  FUN_004b7a90(&local_18);
  ExceptionList = local_c;
  return;
}


/* [VTable] UGameUISceneClient virtual function #81
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_81(void *this,void *param_1)

{
  int *piVar1;
  int iVar2;
  float *pfVar3;
  int iVar4;
  int local_6c;
  int local_68 [2];
  float fStack_60;
  float fStack_5c;
  undefined4 uStack_58;
  undefined4 uStack_54;
  undefined4 uStack_50;
  undefined4 uStack_4c;
  undefined4 uStack_48;
  undefined1 uStack_44;
  undefined1 uStack_43;
  undefined4 uStack_40;
  undefined4 uStack_3c;
  undefined4 uStack_38;
  undefined4 uStack_34;
  undefined4 uStack_30;
  undefined4 uStack_2c;
  void *pvStack_1c;
  undefined1 *puStack_18;
  undefined4 local_14;
  
  puStack_18 = &LAB_0169b3e4;
  pvStack_1c = ExceptionList;
  iVar4 = 0;
  local_6c = 0;
  local_68[0] = 0;
  local_68[1] = 0;
  local_14 = 1;
  ExceptionList = &pvStack_1c;
  FUN_0067a990(this,&local_6c);
  if (0 < local_68[0]) {
    FUN_005f9190(param_1,(undefined4 *)((int)this + 0x70));
    if (0 < local_68[0]) {
      do {
        iVar2 = *(int *)(local_6c + iVar4 * 4);
        if ((~*(uint *)(iVar2 + 0x5c) & 1) != 0) {
          (**(code **)(*(int *)this + 0x148))(param_1,iVar2);
        }
        iVar4 = iVar4 + 1;
      } while (iVar4 < local_68[0]);
    }
    piVar1 = *(int **)((int)this + 0x44);
    iVar4 = (**(code **)(*piVar1 + 8))();
    iVar2 = (**(code **)(*piVar1 + 4))();
    pfVar3 = FUN_005f5c20(&fStack_60,iVar2,iVar4);
    FUN_005f9190(param_1,pfVar3);
    if ((((*(byte *)((int)this + 200) & 1) != 0) && (*(int *)((int)this + 0xc4) != 0)) &&
       (-1 < *(int *)((int)this + 0x50))) {
      uStack_58 = 0;
      uStack_54 = 0;
      uStack_40 = 0;
      uStack_3c = 0;
      uStack_38 = 0;
      uStack_34 = 0;
      uStack_30 = 0;
      uStack_2c = 0;
      fStack_60 = (float)*(int *)((int)this + 0x4c);
      fStack_5c = (float)*(int *)((int)this + 0x50);
      uStack_44 = 3;
      uStack_43 = 3;
      uStack_50 = DAT_017f7ea0;
      uStack_4c = DAT_017f7ea0;
      uStack_48 = 0;
      (**(code **)(**(int **)((int)this + 0xc4) + 0x10c))(&uStack_58,&uStack_54);
      (**(code **)(**(int **)((int)this + 0xc4) + 0x114))(param_1,local_68);
    }
    if ((*(byte *)((int)this + 0xe8) & 1) != 0) {
      (**(code **)(*(int *)this + 0x184))(param_1);
    }
  }
  local_14 = 0xffffffff;
  FUN_004b7a90(&local_6c);
  ExceptionList = pvStack_1c;
  return;
}


/* [VTable] UGameUISceneClient virtual function #97
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_97(void *this,float *param_1)

{
  undefined **ppuVar1;
  undefined4 *puVar2;
  int *piVar3;
  undefined **unaff_EBX;
  undefined **ppuStack_28;
  int iStack_24;
  int iStack_20;
  undefined8 local_1c;
  void *local_14;
  void *local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0169b40e;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  if (*(int *)((int)this + 0x54) != 0) {
    ExceptionList = &pvStack_c;
    if (DAT_01ee2f18 == (undefined4 *)0x0) {
      ExceptionList = &pvStack_c;
      DAT_01ee2f18 = FUN_006459c0();
      FUN_006477c0();
    }
    for (puVar2 = (undefined4 *)(*(int **)((int)this + 0x54))[0xd]; puVar2 != (undefined4 *)0x0;
        puVar2 = (undefined4 *)puVar2[0xf]) {
      if (puVar2 == DAT_01ee2f18) goto LAB_0067ae32;
    }
    if (DAT_01ee2f18 == (undefined4 *)0x0) {
LAB_0067ae32:
      (**(code **)(**(int **)((int)this + 0x54) + 0x174))(&local_1c);
      uStack_4 = 0;
      FUN_0067e9b0(&ppuStack_28);
      uStack_4 = CONCAT31(uStack_4._1_3_,2);
      FUN_0041b420((int *)&local_1c);
      local_1c._0_4_ = 0;
      local_1c._4_4_ = (void *)0x0;
      local_14 = DAT_017f7ea0;
      local_10 = DAT_017f7ea0;
      ppuVar1 = ppuStack_28;
      if (iStack_24 == 0) {
        ppuVar1 = &PTR_017f8e10;
      }
      USequenceOp__unknown_005f92c0
                (param_1,0.0,0.0,(ushort *)ppuVar1,*(int **)(DAT_01ee1254 + 0x5c),&local_1c,1.0,1.0)
      ;
      uStack_4 = 0xffffffff;
      FUN_0041b420((int *)&ppuStack_28);
      goto LAB_0067af7b;
    }
  }
  local_1c._0_4_ = 0;
  local_1c._4_4_ = (void *)0x0;
  local_14 = DAT_017f7ea0;
  local_10 = DAT_017f7ea0;
  USequenceOp__unknown_005f92c0
            (param_1,0.0,0.0,(ushort *)L"Active: NULL",*(int **)(DAT_01ee1254 + 0x5c),&local_1c,1.0,
             1.0);
LAB_0067af7b:
  if (0 < *(int *)((int)this + 0xbc)) {
    puVar2 = (undefined4 *)FUN_004ad6a0((void *)((int)this + 0xb8),0);
    piVar3 = (int *)UUIScreenObject__unknown_00637830((void *)*puVar2,1,0);
    if (piVar3 != (int *)0x0) {
      (**(code **)(*piVar3 + 0x174))(&local_1c);
      puStack_8 = (undefined1 *)0x3;
      FUN_0067e9b0(&stack0xffffffd4);
      puStack_8 = (undefined1 *)CONCAT31(puStack_8._1_3_,5);
      FUN_0041b420(&iStack_20);
      iStack_20 = 0;
      local_1c._0_4_ = 0;
      local_1c._4_4_ = DAT_017f7ea0;
      local_14 = DAT_017f7ea0;
      if (ppuStack_28 == (undefined **)0x0) {
        unaff_EBX = &PTR_017f8e10;
      }
      USequenceOp__unknown_005f92c0
                (param_1,0.0,DAT_01851a44,(ushort *)unaff_EBX,*(int **)(DAT_01ee1254 + 0x5c),
                 (undefined8 *)&iStack_20,1.0,1.0);
      puStack_8 = (undefined1 *)0xffffffff;
      FUN_0041b420((int *)&stack0xffffffd4);
      ExceptionList = local_10;
      return;
    }
  }
  local_1c._0_4_ = 0;
  local_1c._4_4_ = (void *)0x0;
  local_14 = DAT_017f7ea0;
  local_10 = DAT_017f7ea0;
  USequenceOp__unknown_005f92c0
            (param_1,0.0,DAT_01851a44,(ushort *)L"Focused: NULL",*(int **)(DAT_01ee1254 + 0x5c),
             &local_1c,1.0,1.0);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] UGameUISceneClient virtual function #91
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_91(int *param_1)

{
  (**(code **)(*param_1 + 0x17c))();
  FUN_0067bd10((int)param_1);
                    /* WARNING: Could not recover jumptable at 0x0067c6ef. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (**(code **)(*param_1 + 0x180))();
  return;
}


/* [VTable] UGameUISceneClient virtual function #94
   VTable: vtable_UGameUISceneClient at 01864954 */

undefined4 __thiscall UGameUISceneClient__vfunc_94(void *this,int param_1)

{
  int *this_00;
  undefined4 *puVar1;
  int *piVar2;
  int iVar3;
  int iVar4;
  undefined4 local_4;
  
  iVar4 = DAT_01ee30c0;
  this_00 = (int *)((int)this + 0xb8);
  DAT_01ee30c0 = DAT_01ee30c0 + 1;
  iVar3 = 0;
  local_4 = 0;
  if (0 < *(int *)((int)this + 0xbc)) {
    piVar2 = (int *)*this_00;
    while (*piVar2 != param_1) {
      iVar3 = iVar3 + 1;
      piVar2 = piVar2 + 1;
      if (*(int *)((int)this + 0xbc) <= iVar3) {
        DAT_01ee30c0 = iVar4;
        return 0;
      }
    }
    if ((iVar3 != -1) && (iVar4 = *(int *)((int)this + 0xbc) + -1, iVar3 <= iVar4)) {
      local_4 = 1;
      do {
        if ((((-1 < iVar4) && (iVar4 < *(int *)((int)this + 0xbc))) &&
            (piVar2 = *(int **)(*this_00 + iVar4 * 4), piVar2[0x59] == *(int *)(param_1 + 0x164)))
           && (((piVar2[0x66] & 0x1000U) == 0 || (iVar4 == iVar3)))) {
          if ((char)piVar2[0x66] < '\0') {
            (**(code **)(*(int *)this + 0x14c))(0);
          }
          FUN_0070eb10(this_00,iVar4,1);
          FUN_0067c700((int)this);
          (**(code **)(*piVar2 + 0x214))();
        }
        iVar4 = iVar4 + -1;
      } while (iVar3 <= iVar4);
      if ((DAT_01ee30c0 == 1) &&
         ((**(code **)(*(int *)this + 0x16c))(), 0 < *(int *)((int)this + 0xbc))) {
        puVar1 = (undefined4 *)FUN_004ad6a0(this_00,0);
        (**(code **)(*(int *)*puVar1 + 0x218))(0);
      }
    }
  }
  DAT_01ee30c0 = DAT_01ee30c0 + -1;
  return local_4;
}


/* [VTable] UGameUISceneClient virtual function #92
   VTable: vtable_UGameUISceneClient at 01864954 */

void __thiscall UGameUISceneClient__vfunc_92(void *this,int param_1)

{
  int iVar1;
  
  *(int *)((int)this + 0xcc) = param_1;
  if ((*(uint *)((int)this + 200) & 2) != 0) {
    *(uint *)((int)this + 200) = *(uint *)((int)this + 200) & 0xfffffffd;
    FUN_0067bd10((int)this);
  }
  if ((*(uint *)((int)this + 200) & 4) != 0) {
    *(uint *)((int)this + 200) = *(uint *)((int)this + 200) & 0xfffffffb;
    (**(code **)(*(int *)this + 0x17c))();
  }
  (**(code **)(*(int *)this + 0x168))();
  if ((*(byte *)((int)this + 200) & 1) != 0) {
    FUN_0067c700((int)this);
  }
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0xbc)) {
    do {
      FUN_00634960(*(void **)(*(int *)((int)this + 0xb8) + iVar1 * 4),param_1);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + 0xbc));
  }
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0xbc)) {
    do {
      (**(code **)(**(int **)(*(int *)((int)this + 0xb8) + iVar1 * 4) + 0x220))(param_1);
      iVar1 = iVar1 + 1;
    } while (iVar1 < *(int *)((int)this + 0xbc));
  }
  return;
}


/* [VTable] UGameUISceneClient virtual function #70
   VTable: vtable_UGameUISceneClient at 01864954 */

void __fastcall UGameUISceneClient__vfunc_70(int *param_1)

{
  UUISceneClient__vfunc_70(param_1);
  FUN_0067d560(param_1);
  return;
}


/* [VTable] UGameUISceneClient virtual function #2
   VTable: vtable_UGameUISceneClient at 01864954 */

int * __thiscall UGameUISceneClient__vfunc_2(void *this,byte param_1)

{
  FUN_00680080(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UGameViewportClient.c ========== */

/*
 * SGW.exe - UGameViewportClient (6 functions)
 */

/* [VTable] UGameViewportClient virtual function #1
   VTable: vtable_UGameViewportClient at 01847304 */

bool __fastcall UGameViewportClient__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* Also in vtable: UGameViewportClient (slot 0) */

undefined4 UGameViewportClient__vfunc_0(void)

{
  return 0;
}


/* [VTable] UGameViewportClient virtual function #68
   VTable: vtable_UGameViewportClient at 01847304 */

void __fastcall UGameViewportClient__vfunc_68(int *param_1)

{
  int iVar1;
  undefined4 uVar2;
  undefined4 uVar3;
  undefined4 uVar4;
  
  iVar1 = HVSystemOptionPolicyEnum__unknown_0056f980();
  FUN_0056c830(iVar1);
  iVar1 = *param_1;
  uVar4 = 0;
  uVar3 = 0;
  uVar2 = FUN_0049ffb0(param_1,DAT_01ee146c,DAT_01ee1470,0);
  (**(code **)(iVar1 + 0xe8))(uVar2,uVar3,uVar4);
  if ((int *)param_1[0x17] != (int *)0x0) {
    (**(code **)(*(int *)param_1[0x17] + 0x120))();
  }
  param_1[0x17] = 0;
  param_1[0x1c] = 0;
  FUN_0049f940((int)param_1);
  return;
}


/* [VTable] UGameViewportClient virtual function #69
   VTable: vtable_UGameViewportClient at 01847304 */

void __thiscall UGameViewportClient__vfunc_69(void *this,undefined4 *param_1)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  void *this_00;
  undefined4 uVar4;
  undefined4 uVar5;
  int *piVar6;
  
  iVar1 = *(int *)((int)this + 0x44);
  *(undefined4 **)((int)this + 0x48) = param_1;
  if (param_1 == (undefined4 *)0x0) {
    iVar2 = 0;
  }
  else {
    iVar2 = (**(code **)*param_1)();
  }
  *(int *)((int)this + 0x44) = iVar2;
  if ((iVar1 == 0) && (iVar2 != 0)) {
    iVar1 = *(int *)this;
    uVar5 = 0;
    uVar4 = 0;
    uVar3 = FUN_0049ffb0(this,DAT_01ee159c,DAT_01ee15a0,0);
    (**(code **)(iVar1 + 0xe8))(uVar3,uVar4,uVar5);
  }
  piVar6 = *(int **)((int)this + 0x44);
  this_00 = (void *)HVSystemOptionPolicyEnum__unknown_0056f980();
  FUN_0056d810(this_00,piVar6);
  return;
}


/* [VTable] UGameViewportClient virtual function #67
   VTable: vtable_UGameViewportClient at 01847304 */

void * __thiscall UGameViewportClient__vfunc_67(void *this,undefined4 param_1,void *param_2)

{
  void *pvVar1;
  int aiStack_34 [2];
  undefined **local_2c;
  undefined **local_28;
  undefined **ppuStack_24;
  int local_20 [3];
  void *pvStack_14;
  void *pvStack_c;
  undefined1 *puStack_8;
  void *local_4;
  
  local_4 = (void *)0xffffffff;
  puStack_8 = &LAB_01691b73;
  pvStack_c = ExceptionList;
  aiStack_34[1] = 0;
  ExceptionList = &pvStack_c;
  FUN_00425e70(param_2,aiStack_34 + 2,1000);
  local_4 = (void *)0x1;
  FUN_005da8b0(local_20,*(undefined4 *)((int)this + 0x70));
  local_4 = (void *)CONCAT31(local_4._1_3_,2);
  if (local_28 == (undefined **)0x0) {
    local_2c = &PTR_017f8e10;
  }
  (*(code *)**(undefined4 **)((int)this + 0x40))(local_2c,local_20);
  pvVar1 = local_4;
  if (local_20[0] == 0) {
    ppuStack_24 = &PTR_017f8e10;
  }
  FUN_0041aab0(local_4,(short *)ppuStack_24);
  pvStack_c._1_3_ = (uint3)((uint)pvStack_c >> 8);
  local_28 = FOutputDevice::vftable;
  pvStack_c._0_1_ = 1;
  FUN_0041b420((int *)&ppuStack_24);
  pvStack_c = (void *)((uint)pvStack_c._1_3_ << 8);
  FUN_0041b420(aiStack_34);
  ExceptionList = pvStack_14;
  return pvVar1;
}


/* [VTable] UGameViewportClient virtual function #2
   VTable: vtable_UGameViewportClient at 01847304 */

int * __thiscall UGameViewportClient__vfunc_2(void *this,byte param_1)

{
  FUN_005db970(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== ULocalPlayer.c ========== */

/*
 * SGW.exe - ULocalPlayer (9 functions)
 */

/* Also in vtable: ULocalPlayer (slot 0) */

undefined4 ULocalPlayer__vfunc_0(void)

{
  return 1;
}


/* [VTable] ULocalPlayer virtual function #11
   VTable: vtable_ULocalPlayer at 0184716c */

void __fastcall ULocalPlayer__vfunc_11(int param_1)

{
  int iVar1;
  
  for (iVar1 = param_1; iVar1 != 0; iVar1 = *(int *)(iVar1 + 0x28)) {
    if ((*(uint *)(iVar1 + 8) & 0x600) != 0) goto LAB_005d2a8b;
  }
  (**(code **)**(undefined4 **)(param_1 + 0x88))();
  *(undefined4 *)(param_1 + 0x88) = 0;
LAB_005d2a8b:
  UTestIpDrv__vfunc_11(param_1);
  return;
}


/* [VTable] ULocalPlayer virtual function #70
   VTable: vtable_ULocalPlayer at 0184716c */

undefined4 __thiscall ULocalPlayer__vfunc_70(void *this,int param_1)

{
  if ((-1 < param_1) && (param_1 < *(int *)((int)this + 0x80))) {
    return *(undefined4 *)(*(int *)((int)this + 0x7c) + param_1 * 4);
  }
  return 0;
}


/* [VTable] ULocalPlayer virtual function #1
   VTable: vtable_ULocalPlayer at 0184716c */

bool __fastcall ULocalPlayer__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee2b40 == (undefined4 *)0x0) {
    DAT_01ee2b40 = FUN_005d32b0();
    FUN_005d21e0();
  }
  return puVar1 != DAT_01ee2b40;
}


/* [VTable] ULocalPlayer virtual function #67
   VTable: vtable_ULocalPlayer at 0184716c */

undefined4 __thiscall ULocalPlayer__vfunc_67(void *this,int param_1,int param_2)

{
  int *piVar1;
  int iVar2;
  wchar_t *pwVar3;
  undefined4 uVar4;
  undefined4 uVar5;
  int iVar6;
  undefined4 uVar7;
  
  if (param_1 == 0) {
    return 0;
  }
  uVar7 = 0;
  iVar6 = 0;
  uVar5 = 0xffffffff;
  uVar4 = 0xffffffff;
  pwVar3 = L"None";
  iVar2 = FUN_0049ed80();
  iVar2 = FUN_004ad5a0(param_1,param_1,iVar2,pwVar3,uVar4,uVar5,iVar6,uVar7);
  iVar2 = FUN_005d9a90(iVar2);
  if (iVar2 != 0) {
    if ((param_2 == -1) || (*(int *)((int)this + 0x80) <= param_2)) {
      param_2 = *(int *)((int)this + 0x80);
    }
    FUN_0041a820((int *)((int)this + 0x7c),param_2,1,4,8);
    piVar1 = (int *)(*(int *)((int)this + 0x7c) + param_2 * 4);
    if (piVar1 != (int *)0x0) {
      *piVar1 = iVar2;
    }
    ULocalPlayer__unknown_005d3780((int)this);
    return 1;
  }
  return 0;
}


/* [VTable] ULocalPlayer virtual function #68
   VTable: vtable_ULocalPlayer at 0184716c */

undefined4 __thiscall ULocalPlayer__vfunc_68(void *this,int param_1)

{
  if ((-1 < param_1) && (param_1 < *(int *)((int)this + 0x80))) {
    FUN_0070eb10((void *)((int)this + 0x7c),param_1,1);
    ULocalPlayer__unknown_005d3780((int)this);
    return 1;
  }
  return 0;
}


/* [VTable] ULocalPlayer virtual function #69
   VTable: vtable_ULocalPlayer at 0184716c */

undefined4 __fastcall ULocalPlayer__vfunc_69(int param_1)

{
  int iVar1;
  undefined4 uVar2;
  
  *(undefined4 *)(param_1 + 0x80) = 0;
  if (*(int *)(param_1 + 0x84) != 0) {
    iVar1 = *(int *)(param_1 + 0x7c);
    *(undefined4 *)(param_1 + 0x84) = 0;
    if (iVar1 != 0) {
      if (DAT_01ea5778 == (int *)0x0) {
        FUN_00416660();
      }
      uVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar1,0,8);
      *(undefined4 *)(param_1 + 0x7c) = uVar2;
    }
  }
  ULocalPlayer__unknown_005d3780(param_1);
  return 1;
}


/* [VTable] ULocalPlayer virtual function #71
   VTable: vtable_ULocalPlayer at 0184716c */

void __fastcall ULocalPlayer__vfunc_71(int param_1)

{
  undefined4 *puVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  bool bVar5;
  int iVar6;
  int *piVar7;
  int iVar8;
  undefined4 uVar9;
  undefined4 uVar10;
  uint uVar11;
  uint uVar12;
  undefined4 uVar13;
  undefined4 uVar14;
  int iVar15;
  int iStack_c;
  int iStack_8;
  
  if (*(int *)(param_1 + 0x80) == 0) {
    *(undefined4 *)(param_1 + 0x78) = 0;
    return;
  }
  iVar15 = 0;
  uVar14 = 0;
  uVar13 = 0;
  uVar12 = 0;
  uVar11 = 0;
  uVar9 = 0;
  uVar10 = 0;
  if (DAT_01ee4374 == (undefined4 *)0x0) {
    DAT_01ee4374 = FUN_00850ac0();
    FUN_00850740();
  }
  puVar1 = DAT_01ee4374;
  iVar6 = FUN_0049ed80();
  iVar15 = T_StaticClass_17(puVar1,iVar6,uVar9,uVar10,uVar11,uVar12,uVar13,uVar14,iVar15);
  *(int *)(param_1 + 0x78) = iVar15;
  if (iVar15 == 0) {
    FUN_00486000("PlayerPostProcess",".\\Src\\UnPlayer.cpp",0x7ea);
  }
  bVar5 = false;
  iStack_8 = 0;
  if (0 < *(int *)(param_1 + 0x80)) {
    do {
      iVar15 = *(int *)(*(int *)(param_1 + 0x7c) + iStack_8 * 4);
      if ((iVar15 != 0) && (iStack_c = 0, 0 < *(int *)(iVar15 + 0x40))) {
        do {
          iVar6 = *(int *)(*(int *)(iVar15 + 0x3c) + iStack_c * 4);
          if (iVar6 != 0) {
            if (DAT_01ee4838 == (undefined4 *)0x0) {
              DAT_01ee4838 = FUN_00856fb0();
              FUN_00856e50();
            }
            for (puVar1 = *(undefined4 **)(iVar6 + 0x34); puVar1 != (undefined4 *)0x0;
                puVar1 = (undefined4 *)puVar1[0xf]) {
              if (puVar1 == DAT_01ee4838) goto LAB_005d3897;
            }
            if (DAT_01ee4838 == (undefined4 *)0x0) {
LAB_005d3897:
              if (!bVar5) {
                piVar7 = (int *)FUN_005da280(4,(int *)(*(int *)(param_1 + 0x78) + 0x3c));
                if (piVar7 != (int *)0x0) {
                  *piVar7 = iVar6;
                }
                bVar5 = true;
              }
            }
            else {
              iVar2 = *(int *)(param_1 + 0x78);
              iVar3 = *(int *)(iVar2 + 0x40);
              piVar7 = (int *)(iVar2 + 0x3c);
              iVar8 = iVar3 + 1;
              *(int *)(iVar2 + 0x40) = iVar8;
              if (*(int *)(iVar2 + 0x44) < iVar8) {
                iVar4 = *piVar7;
                iVar8 = ((int)(iVar8 * 3 + (iVar8 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar8;
                *(int *)(iVar2 + 0x44) = iVar8;
                if ((iVar4 != 0) || (iVar8 != 0)) {
                  if (DAT_01ea5778 == (int *)0x0) {
                    FUN_00416660();
                  }
                  iVar8 = (**(code **)(*DAT_01ea5778 + 8))(iVar4,iVar8 * 4,8);
                  *piVar7 = iVar8;
                }
              }
              piVar7 = (int *)(*piVar7 + iVar3 * 4);
              if (piVar7 != (int *)0x0) {
                *piVar7 = iVar6;
              }
            }
          }
          iStack_c = iStack_c + 1;
        } while (iStack_c < *(int *)(iVar15 + 0x40));
      }
      iStack_8 = iStack_8 + 1;
    } while (iStack_8 < *(int *)(param_1 + 0x80));
  }
  return;
}


/* [VTable] ULocalPlayer virtual function #2
   VTable: vtable_ULocalPlayer at 0184716c */

int * __thiscall ULocalPlayer__vfunc_2(void *this,byte param_1)

{
  FUN_005db520(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UPlayer.c ========== */

/*
 * SGW.exe - UPlayer (4 functions)
 */

/* [VTable] UPlayer virtual function #2
   VTable: vtable_UPlayer at 01800c24 */

int * __thiscall UPlayer__vfunc_2(void *this,byte param_1)

{
  FUN_0047f420(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}


/* [VTable] UPlayer virtual function #1
   VTable: vtable_UPlayer at 01800c24 */

bool __fastcall UPlayer__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* Also in vtable: UPlayer (slot 0) */

undefined4 UPlayer__vfunc_0(void)

{
  return 1;
}


/* WARNING: Variable defined which should be unmapped: param_1 */
/* [VTable] UPlayer virtual function #67
   VTable: vtable_UPlayer at 01800c24 */

void UPlayer__vfunc_67(int *param_1,int *param_2)

{
  undefined4 *unaff_EBX;
  int unaff_EBP;
  undefined8 *unaff_ESI;
  void *unaff_EDI;
  int *unaff_retaddr;
  void *in_stack_00000098;
  
  FUN_00683ba0(unaff_retaddr,param_1,param_2);
  FUN_0068a500(unaff_EDI,*(int **)(unaff_EBP + 8),unaff_ESI,*unaff_EBX);
  ExceptionList = in_stack_00000098;
  return;
}




/* ========== UPlayerInput.c ========== */

/*
 * SGW.exe - UPlayerInput (5 functions)
 */

/* Also in vtable: UPlayerInput (slot 0) */

undefined4 UPlayerInput__vfunc_0(void)

{
  return 1;
}


/* [VTable] UPlayerInput virtual function #75
   VTable: vtable_UPlayerInput at 018cab7c */

void __thiscall UPlayerInput__vfunc_75(void *this,float *param_1,float param_2)

{
  int iVar1;
  
  if (param_2 != (float)PTR_017f94b8) {
    iVar1 = *(int *)((int)this + 0x90);
    if (((((iVar1 != DAT_01ee2340) || (*(int *)((int)this + 0x94) != DAT_01ee2344)) &&
         ((iVar1 != DAT_01ee2348 || (*(int *)((int)this + 0x94) != DAT_01ee234c)))) &&
        ((iVar1 != DAT_01ee2350 || (*(int *)((int)this + 0x94) != DAT_01ee2354)))) &&
       ((iVar1 != DAT_01ee2358 || (*(int *)((int)this + 0x94) != DAT_01ee235c)))) {
      *(uint *)((int)this + 0x8c) = *(uint *)((int)this + 0x8c) & 0xfffffffe;
      *param_1 = *param_1 + param_2;
      return;
    }
    *(uint *)((int)this + 0x8c) = *(uint *)((int)this + 0x8c) | 1;
  }
  *param_1 = *param_1 + param_2;
  return;
}


/* [VTable] UPlayerInput virtual function #74
   VTable: vtable_UPlayerInput at 018cab7c */

void __fastcall UPlayerInput__vfunc_74(int *param_1)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  int iVar4;
  undefined4 *puVar5;
  undefined4 uVar6;
  int local_34;
  int local_30;
  undefined4 local_28;
  undefined4 local_24;
  undefined4 local_20;
  undefined1 local_1c;
  undefined4 local_18;
  undefined4 local_14;
  undefined4 local_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016b640a;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar2 = FUN_00716d30(*(int *)(param_1[10] + 0x2ac));
  if (iVar2 != 0) {
    FUN_009720a0(&local_34,param_1 + 0x15);
    local_4 = 0;
    iVar4 = 0;
    if (0 < local_30) {
      do {
        local_28 = *(undefined4 *)(iVar2 + 0x60);
        local_24 = *(undefined4 *)(local_34 + iVar4 * 8);
        local_20 = *(undefined4 *)(local_34 + 4 + iVar4 * 8);
        uVar6 = 0;
        puVar5 = &local_28;
        local_10 = 0;
        local_14 = 0;
        iVar1 = *param_1;
        local_1c = 1;
        local_18 = 0;
        uVar3 = FUN_0049ffb0(param_1,DAT_01ee152c,DAT_01ee1530,0);
        (**(code **)(iVar1 + 0xe8))(uVar3,puVar5,uVar6);
        iVar4 = iVar4 + 1;
      } while (iVar4 < local_30);
    }
    local_4 = 0xffffffff;
    FUN_00b98f40(&local_34);
  }
  param_1[0x16] = 0;
  if (param_1[0x17] != 0) {
    iVar2 = param_1[0x15];
    param_1[0x17] = 0;
    if (iVar2 != 0) {
      if (DAT_01ea5778 == (int *)0x0) {
        FUN_00416660();
      }
      iVar2 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,0,8);
      param_1[0x15] = iVar2;
    }
  }
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] UPlayerInput virtual function #1
   VTable: vtable_UPlayerInput at 018cab7c */

bool __fastcall UPlayerInput__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01ee4334 == (undefined4 *)0x0) {
    DAT_01ee4334 = FUN_00849e70();
    FUN_00848930();
  }
  return puVar1 != DAT_01ee4334;
}


/* [VTable] UPlayerInput virtual function #2
   VTable: vtable_UPlayerInput at 018cab7c */

int * __thiscall UPlayerInput__vfunc_2(void *this,byte param_1)

{
  FUN_0084a750(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UPlayerInputMask_CustomDrawProxy.c ========== */

/*
 * SGW.exe - UPlayerInputMask_CustomDrawProxy (1 functions)
 */

undefined4 UPlayerInputMask_CustomDrawProxy__vfunc_0(void)

{
  return 1;
}




/* ========== UPlayerInputMask_CustomInputProxy.c ========== */

/*
 * SGW.exe - UPlayerInputMask_CustomInputProxy (1 functions)
 */

undefined4 UPlayerInputMask_CustomInputProxy__vfunc_0(void)

{
  return 1;
}




/* ========== USettings.c ========== */

/*
 * SGW.exe - USettings (36 functions)
 */

void __fastcall USettings__unknown_008d5c10(char *param_1)

{
  if (*param_1 == '\x04') {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 *)(param_1 + 8));
  }
  if (*param_1 == '\x06') {
                    /* WARNING: Subroutine does not return */
    scalable_free(*(undefined4 *)(param_1 + 8));
  }
  *param_1 = '\0';
  param_1[4] = '\0';
  param_1[5] = '\0';
  param_1[6] = '\0';
  param_1[7] = '\0';
  param_1[8] = '\0';
  param_1[9] = '\0';
  param_1[10] = '\0';
  param_1[0xb] = '\0';
  return;
}


/* [VTable] USettings virtual function #101
   VTable: vtable_USettings at 018ebe54 */

void __fastcall USettings__vfunc_101(int param_1,undefined4 param_2)

{
  int *piVar1;
  int unaff_EBP;
  float in_XMM1_Da;
  float in_XMM2_Da;
  float in_XMM3_Da;
  float in_XMM4_Da;
  float in_XMM5_Da;
  float in_XMM6_Da;
  float fVar2;
  float fVar3;
  float in_XMM7_Da;
  byte bStack00000014;
  undefined1 uStack00000015;
  undefined1 uStack00000016;
  undefined1 uStack00000017;
  float fStack00000020;
  float fStack00000024;
  float fStack00000028;
  undefined4 uStack0000005c;
  float fStack0000006c;
  float fStack00000070;
  float fStack00000074;
  float fStack00000078;
  float fStack0000007c;
  float in_stack_00000080;
  undefined8 uStack00000084;
  float fStack0000008c;
  undefined1 *puStack_9c;
  undefined1 **ppuStack_98;
  undefined1 **ppuStack_90;
  undefined1 **ppuStack_8c;
  undefined1 **ppuStack_88;
  undefined1 *puStack_80;
  undefined1 **ppuStack_7c;
  undefined1 **ppuStack_78;
  undefined1 *puStack_70;
  undefined1 *puStack_6c;
  undefined1 **ppuStack_68;
  undefined1 *puStack_60;
  undefined1 *puStack_5c;
  undefined1 **ppuStack_58;
  undefined1 *puStack_50;
  undefined1 *puStack_4c;
  undefined1 **ppuStack_48;
  undefined1 *puStack_40;
  undefined1 *puStack_30;
  undefined1 *puStack_2c;
  undefined1 *puStack_20;
  undefined1 *puStack_1c;
  undefined1 *puStack_18;
  undefined1 *puStack_10;
  undefined1 *puStack_c;
  undefined1 *puStack_8;
  
  fStack00000074 =
       *(float *)(param_1 + 0x48) * in_XMM6_Da + *(float *)(param_1 + 0x38) * fStack00000070 +
       *(float *)(param_1 + 0x28) * fStack0000006c + *(float *)(param_1 + 0x58);
  _fStack0000006c =
       CONCAT44(*(float *)(param_1 + 0x44) * in_XMM6_Da + in_XMM7_Da * in_XMM2_Da +
                *(float *)(param_1 + 0x24) * fStack0000006c + *(float *)(param_1 + 0x54),
                *(float *)(param_1 + 0x40) * in_XMM6_Da + in_XMM1_Da +
                fStack0000006c * *(float *)(param_1 + 0x20) + *(float *)(param_1 + 0x50));
  fVar2 = *(float *)(param_1 + 0x38) * fStack0000007c;
  fVar3 = *(float *)(param_1 + 0x28) * fStack00000078;
  _fStack00000078 =
       CONCAT44(*(float *)(param_1 + 0x44) * in_stack_00000080 +
                *(float *)(param_1 + 0x34) * fStack0000007c +
                *(float *)(param_1 + 0x24) * fStack00000078 + *(float *)(param_1 + 0x54),
                *(float *)(param_1 + 0x40) * in_stack_00000080 +
                *(float *)(param_1 + 0x30) * fStack0000007c +
                fStack00000078 * *(float *)(param_1 + 0x20) + *(float *)(param_1 + 0x50));
  in_stack_00000080 =
       *(float *)(param_1 + 0x48) * in_stack_00000080 + fVar2 + fVar3 + *(float *)(param_1 + 0x58);
  fStack00000020 =
       *(float *)(param_1 + 0x40) * in_XMM3_Da + *(float *)(param_1 + 0x30) * in_XMM5_Da +
       in_XMM4_Da * *(float *)(param_1 + 0x20) + *(float *)(param_1 + 0x50);
  fStack00000024 =
       *(float *)(param_1 + 0x44) * in_XMM3_Da + *(float *)(param_1 + 0x34) * in_XMM5_Da +
       *(float *)(param_1 + 0x24) * in_XMM4_Da + *(float *)(param_1 + 0x54);
  fStack00000028 =
       *(float *)(param_1 + 0x48) * in_XMM3_Da + *(float *)(param_1 + 0x38) * in_XMM5_Da +
       *(float *)(param_1 + 0x28) * in_XMM4_Da + *(float *)(param_1 + 0x58);
  uStack00000084 = CONCAT44(fStack00000024,fStack00000020);
  bStack00000014 = 0xff;
  uStack00000015 = 200;
  uStack00000016 = 0x96;
  uStack00000017 = 0xff;
  puStack_8 = (undefined1 *)0x620288;
  uStack0000005c = param_2;
  fStack0000008c = fStack00000028;
  FUN_004f6f20(&stack0x00000020,&stack0x00000014);
  piVar1 = *(int **)(unaff_EBP + 8);
  puStack_8 = (undefined1 *)&stack0x00000020;
  puStack_c = &stack0x0000003c;
  puStack_10 = &stack0x00000030;
  (**(code **)(*piVar1 + 0x10))();
  puStack_18 = (undefined1 *)0x6202b2;
  FUN_004f6f20(&stack0x00000010,&stack0x00000004);
  puStack_18 = &stack0x00000010;
  puStack_1c = &stack0x00000038;
  puStack_20 = &stack0x0000002c;
  (**(code **)(*piVar1 + 0x10))();
  FUN_004f6f20(&stack0x00000000,(byte *)&puStack_c);
  puStack_2c = &stack0x00000034;
  puStack_30 = (undefined1 *)&stack0x00000028;
  (**(code **)(*piVar1 + 0x10))();
  FUN_004f6f20(&puStack_10,(byte *)&puStack_1c);
  puStack_40 = (undefined1 *)&stack0x00000024;
  (**(code **)(*piVar1 + 0x10))();
  ppuStack_48 = (undefined1 **)0x620327;
  FUN_004f6f20(&puStack_20,(byte *)&puStack_2c);
  ppuStack_48 = &puStack_20;
  puStack_4c = &stack0x0000002c;
  puStack_50 = (undefined1 *)&stack0x00000020;
  (**(code **)(*piVar1 + 0x10))();
  ppuStack_58 = (undefined1 **)0x62034e;
  FUN_004f6f20(&puStack_30,&stack0xffffffc4);
  ppuStack_58 = &puStack_30;
  puStack_5c = (undefined1 *)&stack0x00000028;
  puStack_60 = &stack0x0000001c;
  (**(code **)(*piVar1 + 0x10))();
  ppuStack_68 = (undefined1 **)0x620378;
  FUN_004f6f20(&puStack_40,(byte *)&puStack_4c);
  ppuStack_68 = &puStack_40;
  puStack_6c = (undefined1 *)&stack0x00000024;
  puStack_70 = &stack0x00000018;
  (**(code **)(*piVar1 + 0x10))();
  ppuStack_78 = (undefined1 **)0x6203a5;
  FUN_004f6f20(&puStack_50,(byte *)&puStack_5c);
  ppuStack_78 = &puStack_50;
  ppuStack_7c = &puStack_10;
  puStack_80 = &stack0x00000014;
  (**(code **)(*piVar1 + 0x10))();
  ppuStack_88 = (undefined1 **)0x6203cf;
  FUN_004f6f20(&puStack_60,(byte *)&puStack_6c);
  ppuStack_88 = &puStack_60;
  ppuStack_8c = &puStack_20;
  ppuStack_90 = &puStack_50;
  (**(code **)(*piVar1 + 0x10))();
  ppuStack_98 = (undefined1 **)0x6203f6;
  FUN_004f6f20(&puStack_70,(byte *)&ppuStack_7c);
  ppuStack_98 = &puStack_70;
  puStack_9c = &stack0xffffffdc;
  (**(code **)(*piVar1 + 0x10))(&stack0xffffffac);
  FUN_004f6f20(&puStack_80,(byte *)&ppuStack_8c);
  (**(code **)(*piVar1 + 0x10))(&ppuStack_58,&stack0xffffffd8,&puStack_80);
  FUN_004f6f20(&ppuStack_90,(byte *)&puStack_9c);
  (**(code **)(*piVar1 + 0x10))(&puStack_5c,&puStack_2c,&ppuStack_90);
  ExceptionList = puStack_2c;
  return;
}


/* [VTable] USettings virtual function #70
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_70(void *this,undefined4 param_1,undefined4 param_2)

{
  int iVar1;
  undefined4 unaff_retaddr;
  undefined4 *puVar2;
  
  puVar2 = &param_1;
  iVar1 = (**(code **)(*(int *)this + 0x120))(param_1,param_2,puVar2);
  if (iVar1 != 0) {
    (**(code **)(*(int *)this + 0x10c))(puVar2,unaff_retaddr,param_1);
  }
  return;
}


/* [VTable] USettings virtual function #71
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_71(void *this,undefined4 param_1,undefined4 param_2)

{
  int iVar1;
  undefined4 uVar2;
  undefined4 unaff_retaddr;
  undefined4 *puVar3;
  
  puVar3 = &param_1;
  iVar1 = (**(code **)(*(int *)this + 0x120))(param_1,param_2,puVar3);
  if (iVar1 != 0) {
    uVar2 = (**(code **)(*(int *)this + 0x110))(puVar3,unaff_retaddr);
    return uVar2;
  }
  return 0;
}


/* Also in vtable: USettings (slot 0) */

undefined4 USettings__vfunc_0(void)

{
  return 1;
}


/* [VTable] USettings virtual function #1
   VTable: vtable_USettings at 018ebe54 */

bool __fastcall USettings__vfunc_1(int param_1)

{
  undefined4 *puVar1;
  
  puVar1 = *(undefined4 **)(*(int *)(param_1 + 0x34) + 0x3c);
  if (DAT_01edc680 == (undefined4 *)0x0) {
    DAT_01edc680 = FUN_004a0450();
    FUN_0049fb30();
  }
  return puVar1 != DAT_01edc680;
}


/* [VTable] USettings virtual function #68
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_68(void *this,int param_1,int *param_2)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x40)) {
    piVar2 = *(int **)((int)this + 0x3c);
    do {
      if (*piVar2 == param_1) {
        *param_2 = (*(int **)((int)this + 0x3c))[iVar1 * 3 + 1];
        return 1;
      }
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 3;
    } while (iVar1 < *(int *)((int)this + 0x40));
  }
  return 0;
}


/* [VTable] USettings virtual function #72
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_72(void *this,int param_1,int param_2,undefined4 *param_3)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x58)) {
    piVar2 = (int *)(*(int *)((int)this + 0x54) + 4);
    do {
      if ((*piVar2 == param_1) && (piVar2[1] == param_2)) {
        *param_3 = *(undefined4 *)(*(int *)((int)this + 0x54) + iVar1 * 0x18);
        return 1;
      }
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 6;
    } while (iVar1 < *(int *)((int)this + 0x58));
  }
  return 0;
}


/* [VTable] USettings virtual function #73
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_73(void *this,int *param_1,int param_2)

{
  int *piVar1;
  int iVar2;
  int iVar3;
  int *piVar4;
  
  iVar3 = 0;
  if (0 < *(int *)((int)this + 0x58)) {
    piVar1 = *(int **)((int)this + 0x54);
    piVar4 = piVar1;
    do {
      if (*piVar4 == param_2) {
        iVar2 = piVar1[iVar3 * 6 + 2];
        *param_1 = piVar1[iVar3 * 6 + 1];
        param_1[1] = iVar2;
        return;
      }
      iVar3 = iVar3 + 1;
      piVar4 = piVar4 + 6;
    } while (iVar3 < *(int *)((int)this + 0x58));
  }
  *param_1 = 0;
  param_1[1] = 0;
  return;
}


/* [VTable] USettings virtual function #75
   VTable: vtable_USettings at 018ebe54 */

undefined4 * __thiscall USettings__vfunc_75(void *this,undefined4 *param_1,int param_2,int param_3)

{
  undefined4 uVar1;
  int iVar2;
  int *piVar3;
  int *piVar4;
  int iVar5;
  
  iVar5 = 0;
  if (0 < *(int *)((int)this + 0x58)) {
    piVar4 = *(int **)((int)this + 0x54);
    do {
      if (*piVar4 == param_2) {
        iVar2 = 0;
        if (0 < piVar4[4]) {
          piVar3 = (int *)piVar4[3];
          do {
            if (*piVar3 == param_3) {
              iVar5 = (*(int **)((int)this + 0x54))[iVar5 * 6 + 3];
              uVar1 = *(undefined4 *)(iVar5 + 8 + iVar2 * 0x10);
              *param_1 = *(undefined4 *)(iVar5 + 4 + iVar2 * 0x10);
              param_1[1] = uVar1;
              return param_1;
            }
            iVar2 = iVar2 + 1;
            piVar3 = piVar3 + 4;
          } while (iVar2 < piVar4[4]);
        }
      }
      iVar5 = iVar5 + 1;
      piVar4 = piVar4 + 6;
    } while (iVar5 < *(int *)((int)this + 0x58));
  }
  *param_1 = 0;
  param_1[1] = 0;
  return param_1;
}


/* [VTable] USettings virtual function #74
   VTable: vtable_USettings at 018ebe54 */

uint __thiscall USettings__vfunc_74(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  int *piVar3;
  int iVar4;
  int *piVar5;
  int local_10;
  
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x58)) {
    local_10 = 0;
    piVar3 = *(int **)((int)this + 0x54);
    do {
      if (*piVar3 == param_1) {
        iVar4 = -1;
        iVar1 = 0;
        if (0 < *(int *)((int)this + 0x40)) {
          piVar5 = (int *)(*(int *)((int)this + 0x3c) + local_10);
          do {
            if (param_1 == *piVar5) {
              iVar4 = piVar5[1];
              break;
            }
            iVar1 = iVar1 + 1;
          } while (iVar1 < *(int *)((int)this + 0x40));
        }
        iVar1 = 0;
        if (0 < piVar3[4]) {
          piVar5 = (int *)piVar3[3];
          do {
            if (*piVar5 == iVar4) {
              return *(uint *)((*(int **)((int)this + 0x54))[iVar2 * 6 + 3] + 0xc + iVar1 * 0x10) &
                     1;
            }
            iVar1 = iVar1 + 1;
            piVar5 = piVar5 + 4;
          } while (iVar1 < piVar3[4]);
        }
      }
      local_10 = local_10 + 0xc;
      iVar2 = iVar2 + 1;
      piVar3 = piVar3 + 6;
    } while (iVar2 < *(int *)((int)this + 0x58));
  }
  return 0;
}


/* [VTable] USettings virtual function #76
   VTable: vtable_USettings at 018ebe54 */

undefined4 * __thiscall
USettings__vfunc_76(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  int iVar1;
  int *piVar2;
  int unaff_EBX;
  undefined4 *unaff_ESI;
  
  iVar1 = (**(code **)(*(int *)this + 0x120))(param_2,param_3,&param_2);
  if (iVar1 != 0) {
    iVar1 = 0;
    if (0 < *(int *)((int)this + 0x40)) {
      piVar2 = *(int **)((int)this + 0x3c);
      do {
        if (*piVar2 == unaff_EBX) {
          (**(code **)(*(int *)this + 300))
                    (unaff_ESI,unaff_EBX,(*(int **)((int)this + 0x3c))[iVar1 * 3 + 1]);
          return unaff_ESI;
        }
        iVar1 = iVar1 + 1;
        piVar2 = piVar2 + 3;
      } while (iVar1 < *(int *)((int)this + 0x40));
    }
  }
  *unaff_ESI = 0;
  unaff_ESI[1] = 0;
  return unaff_ESI;
}


/* [VTable] USettings virtual function #78
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_78(void *this,int param_1,int param_2,undefined4 *param_3)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 100)) {
    piVar2 = (int *)(*(int *)((int)this + 0x60) + 4);
    do {
      if ((*piVar2 == param_1) && (piVar2[1] == param_2)) {
        *param_3 = *(undefined4 *)(*(int *)((int)this + 0x60) + iVar1 * 0x24);
        return 1;
      }
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 9;
    } while (iVar1 < *(int *)((int)this + 100));
  }
  return 0;
}


/* [VTable] USettings virtual function #79
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_79(void *this,int *param_1,int param_2)

{
  int *piVar1;
  int iVar2;
  int iVar3;
  int *piVar4;
  
  iVar3 = 0;
  if (0 < *(int *)((int)this + 100)) {
    piVar1 = *(int **)((int)this + 0x60);
    piVar4 = piVar1;
    do {
      if (*piVar4 == param_2) {
        iVar2 = piVar1[iVar3 * 9 + 2];
        *param_1 = piVar1[iVar3 * 9 + 1];
        param_1[1] = iVar2;
        return;
      }
      iVar3 = iVar3 + 1;
      piVar4 = piVar4 + 9;
    } while (iVar3 < *(int *)((int)this + 100));
  }
  *param_1 = 0;
  param_1[1] = 0;
  return;
}


/* [VTable] USettings virtual function #77
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_77(void *this,int param_1,int param_2,undefined4 *param_3)

{
  undefined **ppuVar1;
  int iVar2;
  int *piVar3;
  int *piVar4;
  int *piVar5;
  int iVar6;
  int local_8;
  int local_4;
  
  if (param_3[1] == 0) {
    ppuVar1 = &PTR_017f8e10;
  }
  else {
    ppuVar1 = (undefined **)*param_3;
  }
  FUN_0049e960(&local_8,(wchar_t *)ppuVar1,1);
  param_3 = (undefined4 *)0x0;
  if (0 < *(int *)((int)this + 0x58)) {
    piVar5 = *(int **)((int)this + 0x54);
    do {
      if (((piVar5[1] == param_1) && (piVar5[2] == param_2)) && (iVar6 = 0, 0 < piVar5[4])) {
        piVar4 = (int *)(piVar5[3] + 4);
        do {
          if ((*piVar4 == local_8) && (piVar4[1] == local_4)) {
            iVar2 = 0;
            if (0 < *(int *)((int)this + 0x40)) {
              piVar3 = *(int **)((int)this + 0x3c);
              do {
                if (*piVar3 == *piVar5) {
                  *(undefined4 *)(*(int *)((int)this + 0x3c) + 4 + iVar2 * 0xc) =
                       *(undefined4 *)
                        (iVar6 * 0x10 +
                        *(int *)(*(int *)((int)this + 0x54) + 0xc + (int)param_3 * 0x18));
                  return 1;
                }
                iVar2 = iVar2 + 1;
                piVar3 = piVar3 + 3;
              } while (iVar2 < *(int *)((int)this + 0x40));
            }
          }
          iVar6 = iVar6 + 1;
          piVar4 = piVar4 + 4;
        } while (iVar6 < piVar5[4]);
      }
      param_3 = (undefined4 *)((int)param_3 + 1);
      piVar5 = piVar5 + 6;
    } while ((int)param_3 < *(int *)((int)this + 0x58));
  }
  return 0;
}


/* [VTable] USettings virtual function #82
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_82(void *this,undefined4 param_1,undefined4 param_2)

{
  int iVar1;
  undefined4 uVar2;
  int *piVar3;
  int unaff_EDI;
  undefined4 *unaff_retaddr;
  
  iVar1 = (**(code **)(*(int *)this + 0x138))(param_1,param_2,&param_1);
  if (iVar1 != 0) {
    iVar1 = 0;
    if (0 < *(int *)((int)this + 0x4c)) {
      piVar3 = *(int **)((int)this + 0x48);
      do {
        if (*piVar3 == unaff_EDI) {
          uVar2 = FUN_008d6360(*(int **)((int)this + 0x48) + iVar1 * 5 + 1,unaff_retaddr);
          return uVar2;
        }
        iVar1 = iVar1 + 1;
        piVar3 = piVar3 + 5;
      } while (iVar1 < *(int *)((int)this + 0x4c));
    }
  }
  return 0;
}


/* [VTable] USettings virtual function #83
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_83(void *this,int param_1,int param_2)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    while (*piVar2 != param_1) {
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 5;
      if (*(int *)((int)this + 0x4c) <= iVar1) {
        return;
      }
    }
    if ((char)piVar2[1] == '\x05') {
      if ((char)piVar2[1] == '\x04') {
        iVar1 = piVar2[3];
      }
      else {
        if ((char)piVar2[1] != '\x06') {
          piVar2[3] = 0;
          *(undefined1 *)(piVar2 + 1) = 5;
          piVar2[2] = param_2;
          return;
        }
        iVar1 = piVar2[3];
      }
                    /* WARNING: Subroutine does not return */
      scalable_free(iVar1);
    }
  }
  return;
}


/* [VTable] USettings virtual function #84
   VTable: vtable_USettings at 018ebe54 */

bool __thiscall USettings__vfunc_84(void *this,int param_1,int *param_2)

{
  int *piVar1;
  int iVar2;
  bool bVar3;
  
  bVar3 = false;
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar1 = *(int **)((int)this + 0x48);
    while (*piVar1 != param_1) {
      iVar2 = iVar2 + 1;
      piVar1 = piVar1 + 5;
      if (*(int *)((int)this + 0x4c) <= iVar2) {
        return false;
      }
    }
    bVar3 = (char)piVar1[1] == '\x05';
    if (bVar3) {
      *param_2 = piVar1[2];
    }
  }
  return bVar3;
}


/* [VTable] USettings virtual function #85
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_85(void *this,int param_1,int param_2)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    while (*piVar2 != param_1) {
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 5;
      if (*(int *)((int)this + 0x4c) <= iVar1) {
        return;
      }
    }
    if ((char)piVar2[1] == '\x01') {
      if ((char)piVar2[1] == '\x04') {
        iVar1 = piVar2[3];
      }
      else {
        if ((char)piVar2[1] != '\x06') {
          piVar2[3] = 0;
          *(undefined1 *)(piVar2 + 1) = 1;
          piVar2[2] = param_2;
          return;
        }
        iVar1 = piVar2[3];
      }
                    /* WARNING: Subroutine does not return */
      scalable_free(iVar1);
    }
  }
  return;
}


/* [VTable] USettings virtual function #86
   VTable: vtable_USettings at 018ebe54 */

bool __thiscall USettings__vfunc_86(void *this,int param_1,int *param_2)

{
  int *piVar1;
  int iVar2;
  bool bVar3;
  
  bVar3 = false;
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar1 = *(int **)((int)this + 0x48);
    while (*piVar1 != param_1) {
      iVar2 = iVar2 + 1;
      piVar1 = piVar1 + 5;
      if (*(int *)((int)this + 0x4c) <= iVar2) {
        return false;
      }
    }
    bVar3 = (char)piVar1[1] == '\x01';
    if (bVar3) {
      *param_2 = piVar1[2];
    }
  }
  return bVar3;
}


/* [VTable] USettings virtual function #87
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_87(void *this,int param_1,undefined4 *param_2)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    while (*piVar2 != param_1) {
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 5;
      if (*(int *)((int)this + 0x4c) <= iVar1) {
        return;
      }
    }
    piVar2 = piVar2 + 1;
    if ((char)*piVar2 == '\x04') {
      if (param_2[1] != 0) {
        FUN_008d6000(piVar2,(short *)*param_2);
        return;
      }
      FUN_008d6000(piVar2,(short *)&PTR_017f8e10);
    }
  }
  return;
}


/* [VTable] USettings virtual function #88
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_88(void *this,int param_1,void *param_2)

{
  undefined4 uVar1;
  int *piVar2;
  int iVar3;
  
  uVar1 = 0;
  iVar3 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    while (*piVar2 != param_1) {
      iVar3 = iVar3 + 1;
      piVar2 = piVar2 + 5;
      if (*(int *)((int)this + 0x4c) <= iVar3) {
        return uVar1;
      }
    }
    if ((char)piVar2[1] == '\x04') {
      if ((short *)piVar2[3] != (short *)0x0) {
        FUN_005543f0(param_2,(short *)piVar2[3]);
        return 1;
      }
      FUN_005543f0(param_2,(short *)&DAT_018ebcec);
      uVar1 = 1;
    }
  }
  return uVar1;
}


/* [VTable] USettings virtual function #89
   VTable: vtable_USettings at 018ebe54 */

uint __thiscall USettings__vfunc_89(void *this,int param_1)

{
  uint uVar1;
  int *piVar2;
  
  uVar1 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    do {
      if (*piVar2 == param_1) {
        return CONCAT31((int3)(uVar1 >> 8),(char)piVar2[1]);
      }
      uVar1 = uVar1 + 1;
      piVar2 = piVar2 + 5;
    } while ((int)uVar1 < *(int *)((int)this + 0x4c));
  }
  return uVar1 & 0xffffff00;
}


/* [VTable] USettings virtual function #92
   VTable: vtable_USettings at 018ebe54 */

bool __thiscall USettings__vfunc_92(void *this,int param_1)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    do {
      if (*piVar2 == param_1) goto LAB_008d6951;
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 5;
    } while (iVar1 < *(int *)((int)this + 0x4c));
  }
  piVar2 = (int *)0x0;
LAB_008d6951:
  return piVar2 != (int *)0x0;
}


/* [VTable] USettings virtual function #93
   VTable: vtable_USettings at 018ebe54 */

bool __thiscall USettings__vfunc_93(void *this,int param_1)

{
  int iVar1;
  int *piVar2;
  
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x40)) {
    piVar2 = *(int **)((int)this + 0x3c);
    do {
      if (*piVar2 == param_1) goto LAB_008d6981;
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 3;
    } while (iVar1 < *(int *)((int)this + 0x40));
  }
  piVar2 = (int *)0x0;
LAB_008d6981:
  return piVar2 != (int *)0x0;
}


/* [VTable] USettings virtual function #67
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_67(void *this,int param_1,undefined4 param_2,int param_3)

{
  int iVar1;
  bool bVar2;
  undefined4 uVar3;
  int iVar4;
  int iVar5;
  
  iVar4 = 0;
  bVar2 = false;
  if (0 < *(int *)((int)this + 0x40)) {
    iVar5 = 0;
    do {
      if (*(int *)(*(int *)((int)this + 0x3c) + iVar5) == param_1) {
        *(undefined4 *)(*(int *)((int)this + 0x3c) + iVar5 + 4) = param_2;
        bVar2 = true;
      }
      iVar4 = iVar4 + 1;
      iVar5 = iVar5 + 0xc;
    } while (iVar4 < *(int *)((int)this + 0x40));
    if (bVar2) {
      return;
    }
  }
  if (param_3 == 1) {
    iVar5 = *(int *)((int)this + 0x40);
    iVar4 = iVar5 + 1;
    *(int *)((int)this + 0x40) = iVar4;
    if (*(int *)((int)this + 0x44) < iVar4) {
      iVar1 = *(int *)((int)this + 0x3c);
      iVar4 = ((int)(iVar4 * 3 + (iVar4 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar4;
      *(int *)((int)this + 0x44) = iVar4;
      if ((iVar1 != 0) || (iVar4 != 0)) {
        if (DAT_01ea5778 == (int *)0x0) {
          FUN_00416660();
        }
        uVar3 = (**(code **)(*DAT_01ea5778 + 8))(iVar1,iVar4 * 0xc,8);
        *(undefined4 *)((int)this + 0x3c) = uVar3;
      }
    }
    iVar5 = iVar5 * 0xc;
    *(int *)(iVar5 + *(int *)((int)this + 0x3c)) = param_1;
    *(undefined4 *)(iVar5 + 4 + *(int *)((int)this + 0x3c)) = param_2;
  }
  return;
}


/* [VTable] USettings virtual function #90
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_90(void *this,int *param_1,int param_2)

{
  int iVar1;
  int iVar2;
  int iVar3;
  undefined4 uVar4;
  undefined8 *puVar5;
  int *piVar6;
  int *piVar7;
  int local_c;
  int local_8;
  
  local_8 = 0;
  if (0 < param_1[1]) {
    local_c = 0;
    do {
      iVar1 = *(int *)((int)this + 0x40);
      piVar7 = (int *)(*param_1 + local_c);
      iVar3 = 0;
      if (0 < iVar1) {
        piVar6 = *(int **)((int)this + 0x3c);
        do {
          if (*piVar6 == *piVar7) {
            iVar1 = piVar7[1];
            *piVar6 = *piVar7;
            piVar6[1] = iVar1;
            piVar6[2] = piVar7[2];
            goto LAB_008d6cc1;
          }
          iVar3 = iVar3 + 1;
          piVar6 = piVar6 + 3;
        } while (iVar3 < iVar1);
      }
      if (param_2 != 0) {
        iVar3 = iVar1 + 1;
        *(int *)((int)this + 0x40) = iVar3;
        if (*(int *)((int)this + 0x44) < iVar3) {
          iVar2 = *(int *)((int)this + 0x3c);
          iVar3 = ((int)(iVar3 * 3 + (iVar3 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar3;
          *(int *)((int)this + 0x44) = iVar3;
          if ((iVar2 != 0) || (iVar3 != 0)) {
            if (DAT_01ea5778 == (int *)0x0) {
              FUN_00416660();
            }
            uVar4 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,iVar3 * 0xc,8);
            *(undefined4 *)((int)this + 0x3c) = uVar4;
          }
        }
        puVar5 = (undefined8 *)(*(int *)((int)this + 0x3c) + iVar1 * 0xc);
        *puVar5 = 0;
        *(undefined4 *)(puVar5 + 1) = 0;
        iVar3 = piVar7[1];
        piVar6 = (int *)(*(int *)((int)this + 0x3c) + iVar1 * 0xc);
        *piVar6 = *piVar7;
        piVar6[1] = iVar3;
        piVar6[2] = piVar7[2];
      }
LAB_008d6cc1:
      local_c = local_c + 0xc;
      local_8 = local_8 + 1;
    } while (local_8 < param_1[1]);
  }
  return;
}


/* [VTable] USettings virtual function #69
   VTable: vtable_USettings at 018ebe54 */

undefined4 __thiscall USettings__vfunc_69(void *this,int param_1,int *param_2)

{
  undefined4 *puVar1;
  undefined4 *puVar2;
  int *piVar3;
  int iVar4;
  int iVar5;
  int iVar6;
  int iVar7;
  int *piVar8;
  int local_c;
  
  iVar6 = 0;
  if (0 < *(int *)((int)this + 0x58)) {
    piVar3 = *(int **)((int)this + 0x54);
    piVar8 = piVar3;
    do {
      if (*piVar8 == param_1) {
        piVar8 = piVar3 + iVar6 * 6;
        local_c = 0;
        if (0 < piVar3[iVar6 * 6 + 4]) {
          param_1 = 0;
          do {
            iVar4 = param_2[1];
            puVar2 = (undefined4 *)(piVar8[3] + 4 + param_1);
            iVar7 = iVar4 + 1;
            param_2[1] = iVar7;
            if (param_2[2] < iVar7) {
              iVar5 = *param_2;
              iVar7 = ((int)(iVar7 * 3 + (iVar7 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar7;
              param_2[2] = iVar7;
              if ((iVar5 != 0) || (iVar7 != 0)) {
                if (DAT_01ea5778 == (int *)0x0) {
                  FUN_00416660();
                }
                iVar7 = (**(code **)(*DAT_01ea5778 + 8))(iVar5,iVar7 * 8,8);
                *param_2 = iVar7;
              }
            }
            puVar1 = (undefined4 *)(*param_2 + iVar4 * 8);
            if (puVar1 != (undefined4 *)0x0) {
              *puVar1 = *puVar2;
              puVar1[1] = puVar2[1];
            }
            param_1 = param_1 + 0x10;
            local_c = local_c + 1;
            piVar8 = (int *)(*(int *)((int)this + 0x54) + iVar6 * 0x18);
          } while (local_c < piVar8[4]);
        }
        return 1;
      }
      iVar6 = iVar6 + 1;
      piVar8 = piVar8 + 6;
    } while (iVar6 < *(int *)((int)this + 0x58));
  }
  return 0;
}


/* [VTable] USettings virtual function #80
   VTable: vtable_USettings at 018ebe54 */

undefined4 * __thiscall USettings__vfunc_80(void *this,undefined4 *param_1,int param_2)

{
  int iVar1;
  int *piVar2;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bec77;
  local_c = ExceptionList;
  iVar1 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    piVar2 = *(int **)((int)this + 0x48);
    do {
      if (*piVar2 == param_2) {
        ExceptionList = &local_c;
        USettings__unknown_008d6d00(*(int **)((int)this + 0x48) + iVar1 * 5 + 1,param_1);
        ExceptionList = local_c;
        return param_1;
      }
      iVar1 = iVar1 + 1;
      piVar2 = piVar2 + 5;
    } while (iVar1 < *(int *)((int)this + 0x4c));
  }
  *param_1 = 0;
  param_1[1] = 0;
  param_1[2] = 0;
  return param_1;
}


/* [VTable] USettings virtual function #81
   VTable: vtable_USettings at 018ebe54 */

undefined4 * __thiscall
USettings__vfunc_81(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3)

{
  undefined4 *puVar1;
  int iVar2;
  int *piVar3;
  void *unaff_ESI;
  void *pvStack_c;
  undefined4 *puStack_8;
  int iStack_4;
  
  iStack_4 = -1;
  puStack_8 = (undefined4 *)&LAB_016becaa;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar2 = (**(code **)(*(int *)this + 0x138))(param_2,param_3,&param_2);
  puVar1 = puStack_8;
  if (iVar2 != 0) {
    iVar2 = 0;
    if (0 < *(int *)((int)this + 0x4c)) {
      piVar3 = *(int **)((int)this + 0x48);
      do {
        if (*piVar3 == iStack_4) {
          USettings__unknown_008d6d00(*(int **)((int)this + 0x48) + iVar2 * 5 + 1,puStack_8);
          ExceptionList = unaff_ESI;
          return puVar1;
        }
        iVar2 = iVar2 + 1;
        piVar3 = piVar3 + 5;
      } while (iVar2 < *(int *)((int)this + 0x4c));
    }
  }
  *puStack_8 = 0;
  puStack_8[1] = 0;
  puStack_8[2] = 0;
  ExceptionList = unaff_ESI;
  return puStack_8;
}


/* [VTable] USettings virtual function #91
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_91(void *this,int *param_1,int param_2)

{
  int iVar1;
  int iVar2;
  undefined4 uVar3;
  undefined8 *puVar4;
  int *piVar5;
  int iVar6;
  int *piVar7;
  int local_c;
  int local_8;
  
  local_8 = 0;
  if (0 < param_1[1]) {
    local_c = 0;
    do {
      iVar1 = *(int *)((int)this + 0x4c);
      piVar7 = (int *)(*param_1 + local_c);
      iVar6 = 0;
      if (0 < iVar1) {
        piVar5 = *(int **)((int)this + 0x48);
        do {
          if (*piVar5 == *piVar7) {
            if (piVar7 == piVar5) goto LAB_008d71f4;
            *piVar5 = *piVar7;
            goto LAB_008d71e8;
          }
          iVar6 = iVar6 + 1;
          piVar5 = piVar5 + 5;
        } while (iVar6 < iVar1);
      }
      if (param_2 != 0) {
        iVar6 = iVar1 + 1;
        *(int *)((int)this + 0x4c) = iVar6;
        if (*(int *)((int)this + 0x50) < iVar6) {
          iVar2 = *(int *)((int)this + 0x48);
          iVar6 = ((int)(iVar6 * 3 + (iVar6 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar6;
          *(int *)((int)this + 0x50) = iVar6;
          if ((iVar2 != 0) || (iVar6 != 0)) {
            if (DAT_01ea5778 == (int *)0x0) {
              FUN_00416660();
            }
            uVar3 = (**(code **)(*DAT_01ea5778 + 8))(iVar2,iVar6 * 0x14,8);
            *(undefined4 *)((int)this + 0x48) = uVar3;
          }
        }
        puVar4 = (undefined8 *)(*(int *)((int)this + 0x48) + iVar1 * 0x14);
        *puVar4 = 0;
        puVar4[1] = 0;
        *(undefined4 *)(puVar4 + 2) = 0;
        piVar5 = (int *)(*(int *)((int)this + 0x48) + iVar1 * 0x14);
        if (piVar7 != piVar5) {
          *piVar5 = *piVar7;
LAB_008d71e8:
          FUN_008d69f0(piVar5 + 1,(char *)(piVar7 + 1));
        }
      }
LAB_008d71f4:
      local_c = local_c + 0x14;
      local_8 = local_8 + 1;
    } while (local_8 < param_1[1]);
  }
  return;
}


/* [VTable] USettings virtual function #95
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_95(void *this,int *param_1)

{
  undefined8 *puVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  undefined8 *puVar5;
  int local_8;
  int local_4;
  
  local_8 = 0;
  if (0 < *(int *)((int)this + 0x40)) {
    local_4 = 0;
    do {
      puVar5 = (undefined8 *)(local_4 + *(int *)((int)this + 0x3c));
      if (*(char *)(puVar5 + 1) == '\x02') {
        iVar2 = param_1[1];
        iVar4 = iVar2 + 1;
        param_1[1] = iVar4;
        if (param_1[2] < iVar4) {
          iVar3 = *param_1;
          iVar4 = ((int)(iVar4 * 3 + (iVar4 * 3 >> 0x1f & 7U)) >> 3) + 0x20 + iVar4;
          param_1[2] = iVar4;
          if ((iVar3 != 0) || (iVar4 != 0)) {
            if (DAT_01ea5778 == (int *)0x0) {
              FUN_00416660();
            }
            iVar4 = (**(code **)(*DAT_01ea5778 + 8))(iVar3,iVar4 * 0xc,8);
            *param_1 = iVar4;
          }
        }
        puVar1 = (undefined8 *)(*param_1 + iVar2 * 0xc);
        if (puVar1 != (undefined8 *)0x0) {
          *puVar1 = *puVar5;
          *(undefined4 *)(puVar1 + 1) = *(undefined4 *)(puVar5 + 1);
        }
      }
      local_8 = local_8 + 1;
      local_4 = local_4 + 0xc;
    } while (local_8 < *(int *)((int)this + 0x40));
  }
  return;
}


/* [VTable] USettings virtual function #96
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_96(void *this,int *param_1)

{
  int *this_00;
  int iVar1;
  undefined **ppuVar2;
  wchar_t *pwVar3;
  undefined4 *puVar4;
  int iVar5;
  int iStack_48;
  int iStack_44;
  undefined **ppuStack_3c;
  int iStack_38;
  undefined4 uStack_34;
  int aiStack_30 [3];
  int aiStack_24 [3];
  int aiStack_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_016bed04;
  pvStack_c = ExceptionList;
  iVar5 = 0;
  ExceptionList = &pvStack_c;
  param_1[1] = 0;
  if (param_1[2] != 0) {
    iVar1 = *param_1;
    param_1[2] = 0;
    if (iVar1 != 0) {
      if (DAT_01ea5778 == (int *)0x0) {
        FUN_00416660();
      }
      iVar1 = (**(code **)(*DAT_01ea5778 + 8))(iVar1,0,8);
      *param_1 = iVar1;
    }
  }
  this_00 = *(int **)(*(int *)((int)this + 0x34) + 0x70);
  do {
    if (this_00 == (int *)0x0) {
      if (0 < *(int *)((int)this + 0x40)) {
        iVar1 = 0;
        do {
          (**(code **)(*(int *)this + 0x124))
                    (&iStack_48,*(undefined4 *)(*(int *)((int)this + 0x3c) + iVar1));
          if ((iStack_48 != 0) || (iStack_44 != 0)) {
            FUN_0049b190(&iStack_48,&ppuStack_3c);
            uStack_4 = 5;
            puVar4 = FUN_00fa1ec0(aiStack_30);
            uStack_4._0_1_ = 6;
            if (puVar4[1] == 0) {
              ppuVar2 = &PTR_017f8e10;
            }
            else {
              ppuVar2 = (undefined **)*puVar4;
            }
            FUN_0041b4b0(param_1,(short *)ppuVar2);
            uStack_4 = CONCAT31(uStack_4._1_3_,5);
            FUN_0041b420(aiStack_30);
            uStack_4 = 0xffffffff;
            FUN_0041b420((int *)&ppuStack_3c);
          }
          iVar5 = iVar5 + 1;
          iVar1 = iVar1 + 0xc;
        } while (iVar5 < *(int *)((int)this + 0x40));
      }
      iVar5 = 0;
      iStack_48 = 0;
      if (0 < *(int *)((int)this + 0x4c)) {
        do {
          (**(code **)(*(int *)this + 0x13c))
                    (&ppuStack_3c,*(undefined4 *)(*(int *)((int)this + 0x48) + iVar5));
          if ((ppuStack_3c != (undefined **)0x0) || (iStack_38 != 0)) {
            USettings__unknown_008d6d00((void *)(*(int *)((int)this + 0x48) + iVar5 + 4),aiStack_18)
            ;
            uStack_4 = 7;
            FUN_0049b190(&ppuStack_3c,aiStack_24);
            uStack_4._0_1_ = 8;
            puVar4 = FUN_00493d60(aiStack_30);
            uStack_4._0_1_ = 9;
            if (puVar4[1] == 0) {
              ppuVar2 = &PTR_017f8e10;
            }
            else {
              ppuVar2 = (undefined **)*puVar4;
            }
            FUN_0041b4b0(param_1,(short *)ppuVar2);
            uStack_4._0_1_ = 8;
            FUN_0041b420(aiStack_30);
            uStack_4 = CONCAT31(uStack_4._1_3_,7);
            FUN_0041b420(aiStack_24);
            uStack_4 = 0xffffffff;
            FUN_0041b420(aiStack_18);
          }
          iStack_48 = iStack_48 + 1;
          iVar5 = iVar5 + 0x14;
        } while (iStack_48 < *(int *)((int)this + 0x4c));
      }
      ExceptionList = pvStack_c;
      return;
    }
    if (((this_00[0x13] & 0x40000000U) != 0) && ((*(uint *)(this_00[0xd] + 0xb8) & 0x10000) == 0)) {
      ppuStack_3c = (undefined **)0x0;
      iStack_38 = 0;
      uStack_34 = 0;
      uStack_4 = 2;
      (**(code **)(*this_00 + 0x138))
                (&ppuStack_3c,this_00[0x19] + (int)this,0,this,this_00[0x13] & 1);
      iVar1 = FUN_004add50((int)this_00);
      if (iVar1 == 0) {
LAB_008d73fa:
        FUN_00498ed0(this_00,aiStack_30);
        uStack_4._0_1_ = 3;
        puVar4 = FUN_00493d60(&iStack_48);
        uStack_4._0_1_ = 4;
        if (puVar4[1] == 0) {
          ppuVar2 = &PTR_017f8e10;
        }
        else {
          ppuVar2 = (undefined **)*puVar4;
        }
        FUN_0041b4b0(param_1,(short *)ppuVar2);
        uStack_4._0_1_ = 3;
        FUN_0041b420(&iStack_48);
        uStack_4 = CONCAT31(uStack_4._1_3_,2);
        FUN_0041b420(aiStack_30);
      }
      else {
        ppuVar2 = ppuStack_3c;
        if (iStack_38 == 0) {
          ppuVar2 = &PTR_017f8e10;
        }
        pwVar3 = wcsrchr((wchar_t *)ppuVar2,L' ');
        if (pwVar3 == (wchar_t *)0x0) goto LAB_008d73fa;
      }
      uStack_4 = 0xffffffff;
      FUN_0041b420((int *)&ppuStack_3c);
    }
    this_00 = (int *)this_00[0x1a];
  } while( true );
}


/* [VTable] USettings virtual function #94
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_94(void *this,void *param_1)

{
  undefined4 *puVar1;
  int iVar2;
  int iVar3;
  
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    iVar3 = 0;
    do {
      puVar1 = (undefined4 *)(*(int *)((int)this + 0x48) + iVar3);
      if (*(char *)(puVar1 + 4) == '\x02') {
        FUN_008d7c20(param_1,puVar1);
      }
      iVar2 = iVar2 + 1;
      iVar3 = iVar3 + 0x14;
    } while (iVar2 < *(int *)((int)this + 0x4c));
  }
  return;
}


/* [VTable] USettings virtual function #97
   VTable: vtable_USettings at 018ebe54 */

void __thiscall USettings__vfunc_97(void *this,undefined4 *param_1)

{
  int *this_00;
  undefined **ppuVar1;
  int iVar2;
  short *psVar3;
  undefined4 *puVar4;
  int iVar5;
  wchar_t *_Str;
  int iVar6;
  undefined **local_80;
  int local_7c;
  int aiStack_74 [3];
  int aiStack_68 [3];
  int aiStack_5c [3];
  int local_50 [3];
  int aiStack_44 [4];
  int aiStack_34 [3];
  int aiStack_28 [3];
  int aiStack_1c [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  uint local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_016bed6e;
  pvStack_c = ExceptionList;
  if (param_1[1] == 0) {
    ppuVar1 = &PTR_017f8e10;
  }
  else {
    ppuVar1 = (undefined **)*param_1;
  }
  ExceptionList = &pvStack_c;
  UGameEngine__unknown_005624d0(local_50,(int *)0x0,(wchar_t *)ppuVar1,0);
  local_4 = 0;
  for (this_00 = *(int **)(*(int *)((int)this + 0x34) + 0x70); this_00 != (int *)0x0;
      this_00 = (int *)this_00[0x1a]) {
    if (((this_00[0x13] & 0x40000000U) != 0) && ((*(uint *)(this_00[0xd] + 0xb8) & 0x10000) == 0)) {
      FUN_00498ed0(this_00,&local_80);
      local_4 = CONCAT31(local_4._1_3_,1);
      ppuVar1 = local_80;
      if (local_7c == 0) {
        ppuVar1 = &PTR_017f8e10;
      }
      iVar2 = UGameEngine__unknown_00561a40(local_50,(wchar_t *)ppuVar1);
      if (iVar2 != 0) {
        iVar2 = this_00[0x19];
        ppuVar1 = local_80;
        if (local_7c == 0) {
          ppuVar1 = &PTR_017f8e10;
        }
        psVar3 = (short *)USettings__unknown_00561ac0(local_50,(wchar_t *)ppuVar1,0x18ebd48);
        if (*psVar3 == 0x3d) {
          psVar3 = psVar3 + 1;
        }
        (**(code **)(*this_00 + 0x13c))(psVar3,iVar2 + (int)this,1,this,0);
      }
      local_4 = local_4 & 0xffffff00;
      FUN_0041b420((int *)&local_80);
    }
  }
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x40)) {
    iVar6 = 0;
    do {
      (**(code **)(*(int *)this + 0x124))
                (&local_80,*(undefined4 *)(*(int *)((int)this + 0x3c) + iVar6));
      puVar4 = FUN_0049b190(&local_80,aiStack_74);
      local_4._0_1_ = 2;
      if (puVar4[1] == 0) {
        ppuVar1 = &PTR_017f8e10;
      }
      else {
        ppuVar1 = (undefined **)*puVar4;
      }
      iVar5 = UGameEngine__unknown_00561a40(local_50,(wchar_t *)ppuVar1);
      local_4 = (uint)local_4._1_3_ << 8;
      FUN_0041b420(aiStack_74);
      if (iVar5 != 0) {
        puVar4 = FUN_0049b190(&local_80,aiStack_68);
        local_4._0_1_ = 3;
        if (puVar4[1] == 0) {
          ppuVar1 = &PTR_017f8e10;
        }
        else {
          ppuVar1 = (undefined **)*puVar4;
        }
        _Str = (wchar_t *)USettings__unknown_00561ac0(local_50,(wchar_t *)ppuVar1,0x18ebd4c);
        local_4 = (uint)local_4._1_3_ << 8;
        FUN_0041b420(aiStack_68);
        if (*_Str == L'=') {
          _Str = _Str + 1;
        }
        iVar5 = _wtoi(_Str);
        *(int *)(*(int *)((int)this + 0x3c) + 4 + iVar6) = iVar5;
      }
      iVar2 = iVar2 + 1;
      iVar6 = iVar6 + 0xc;
    } while (iVar2 < *(int *)((int)this + 0x40));
  }
  iVar2 = 0;
  if (0 < *(int *)((int)this + 0x4c)) {
    iVar6 = 0;
    do {
      (**(code **)(*(int *)this + 0x13c))
                (&local_80,*(undefined4 *)(*(int *)((int)this + 0x48) + iVar6));
      puVar4 = FUN_0049b190(&local_80,aiStack_68);
      local_4._0_1_ = 4;
      if (puVar4[1] == 0) {
        ppuVar1 = &PTR_017f8e10;
      }
      else {
        ppuVar1 = (undefined **)*puVar4;
      }
      iVar5 = UGameEngine__unknown_00561a40(local_50,(wchar_t *)ppuVar1);
      local_4 = (uint)local_4._1_3_ << 8;
      FUN_0041b420(aiStack_68);
      if (iVar5 != 0) {
        puVar4 = FUN_0049b190(&local_80,aiStack_5c);
        local_4._0_1_ = 5;
        if (puVar4[1] == 0) {
          ppuVar1 = &PTR_017f8e10;
        }
        else {
          ppuVar1 = (undefined **)*puVar4;
        }
        psVar3 = (short *)USettings__unknown_00561ac0(local_50,(wchar_t *)ppuVar1,0x18ebd50);
        local_4._0_1_ = 0;
        FUN_0041b420(aiStack_5c);
        if (*psVar3 == 0x3d) {
          psVar3 = psVar3 + 1;
        }
        FUN_0041aab0(aiStack_74,psVar3);
        local_4._0_1_ = 6;
        FUN_008d6360((void *)(*(int *)((int)this + 0x48) + 4 + iVar6),aiStack_74);
        local_4 = (uint)local_4._1_3_ << 8;
        FUN_0041b420(aiStack_74);
      }
      iVar2 = iVar2 + 1;
      iVar6 = iVar6 + 0x14;
    } while (iVar2 < *(int *)((int)this + 0x4c));
  }
  local_4 = 10;
  FUN_0041b420(aiStack_1c);
  local_4._0_1_ = 9;
  FUN_0041bb90(aiStack_28);
  local_4._0_1_ = 8;
  FUN_0041b420(aiStack_34);
  local_4 = CONCAT31(local_4._1_3_,7);
  FUN_0041b420(aiStack_44);
  local_4 = 0xffffffff;
  FUN_0041b420(local_50);
  ExceptionList = pvStack_c;
  return;
}


/* [VTable] USettings virtual function #2
   VTable: vtable_USettings at 018ebe54 */

int * __thiscall USettings__vfunc_2(void *this,byte param_1)

{
  FUN_008d80d0(this);
  if ((param_1 & 1) != 0) {
    FUN_0049f210(this);
  }
  return this;
}




/* ========== UnrealMessageCallback.c ========== */

/*
 * SGW.exe - UnrealMessageCallback (2 functions)
 */

/* [VTable] UnrealMessageCallback virtual function #1
   VTable: vtable_UnrealMessageCallback at 018cafc4 */

undefined4 * __thiscall UnrealMessageCallback__vfunc_1(void *this,byte param_1)

{
  void *pvVar1;
  undefined4 *puVar2;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pvVar1 = ExceptionList;
  puStack_8 = &LAB_016b65cf;
  local_c = ExceptionList;
  puVar2 = (undefined4 *)0x0;
  if (this != (void *)0x0) {
    puVar2 = (undefined4 *)((int)this + 4);
  }
  ExceptionList = &local_c;
  *puVar2 = CriticalMessageCallback::vftable;
  local_4 = 0xffffffff;
  *(undefined ***)this = DebugMessageCallback::vftable;
  if ((param_1 & 1) != 0) {
                    /* WARNING: Subroutine does not return */
    scalable_free(this);
  }
  ExceptionList = pvVar1;
  return this;
}


/* Also in vtable: UnrealMessageCallback (slot 0) */

undefined4
UnrealMessageCallback__vfunc_0(undefined4 param_1,undefined4 param_2,char *param_3,va_list param_4)

{
  char cVar1;
  char *pcVar2;
  char local_400 [1024];
  
  _vsnprintf(local_400,0x400,param_3,param_4);
  pcVar2 = local_400;
  do {
    cVar1 = *pcVar2;
    pcVar2 = pcVar2 + 1;
  } while (cVar1 != '\0');
  if (pcVar2[(int)(local_400 + (-1 - (int)(local_400 + 1)))] == '\n') {
    pcVar2[(int)(local_400 + (-1 - (int)(local_400 + 1)))] = '\0';
  }
  return CONCAT31((int3)((uint)(pcVar2 + (int)(local_400 + (-1 - (int)(local_400 + 1)))) >> 8),1);
}



