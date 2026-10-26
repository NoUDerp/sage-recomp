
void __thiscall FUN_00949237(int param_1,int param_2)

{
  int iStack_4;
  
  if (param_2 < 2) {
    iStack_4 = 2;
  }
  else {
    iStack_4 = param_2;
    if (6 < param_2) {
      iStack_4 = 6;
    }
  }
  *(int *)(param_1 + 4) = iStack_4;
  return;
}

