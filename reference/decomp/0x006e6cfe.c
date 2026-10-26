
void __thiscall FUN_006e6cfe(int *param_1,int param_2,int param_3,int param_4)

{
  param_1[2] = param_4;
  if (0 < param_4) {
    param_4 = ((param_3 - param_2) * 0x100) / param_4;
    param_1[1] = param_4;
    *param_1 = param_4 / 2 + param_2 * 0x100;
  }
  return;
}

