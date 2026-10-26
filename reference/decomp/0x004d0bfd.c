
void __thiscall FUN_004d0bfd(float *param_1,float param_2,float param_3,float param_4)

{
  param_1[3] = param_1[1] * param_3 + param_1[2] * param_4 + *param_1 * param_2 + param_1[3];
  param_1[7] = param_1[4] * param_2 + param_1[5] * param_3 + param_1[6] * param_4 + param_1[7];
  param_1[0xb] = param_1[8] * param_2 + param_1[9] * param_3 + param_1[10] * param_4 + param_1[0xb];
  return;
}

