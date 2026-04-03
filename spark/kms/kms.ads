with Interfaces.C;
with System;

package KMS with SPARK_Mode => On is

   pragma Preelaborate;

   subtype Key_Length is Interfaces.C.size_t range 0 .. Interfaces.C.size_t'Last;
   subtype Status_Code is Interfaces.C.int range -1 .. 1;

   STATUS_SUCCESS : constant Status_Code := 0;
   STATUS_ERROR   : constant Status_Code := -1;

   KEY_SIZE       : constant Key_Length := 32;
   SIG_SIZE       : constant Key_Length := 2420;
   MAX_DATA_LEN   : constant Key_Length := 1024 * 1024;

   procedure Generate_Key
     (Key_Out     : System.Address;
      Key_Out_Len : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "kms_generate_key",
          Depends => (Status => (Key_Out, Key_Out_Len)),
          Pre => Key_Out /= System.Null_Address and then Key_Out_Len >= KEY_SIZE,
          Post => (if Status = STATUS_SUCCESS then Key_Out_Len >= KEY_SIZE);

   procedure Sign_Data
     (Data_In     : System.Address;
      Data_In_Len : Interfaces.C.size_t;
      Sig_Out     : System.Address;
      Sig_Out_Len : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "kms_sign",
          Depends => (Status => (Data_In, Data_In_Len, Sig_Out, Sig_Out_Len)),
          Pre => Data_In /= System.Null_Address and then
                 Sig_Out /= System.Null_Address and then
                 Data_In_Len <= MAX_DATA_LEN and then
                 Sig_Out_Len >= SIG_SIZE,
          Post => (if Status = STATUS_SUCCESS then Sig_Out_Len >= SIG_SIZE);

   procedure Verify_Signature
     (Data_In     : System.Address;
      Data_In_Len : Interfaces.C.size_t;
      Sig_In      : System.Address;
      Sig_In_Len  : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "kms_verify",
          Depends => (Status => (Data_In, Data_In_Len, Sig_In, Sig_In_Len)),
          Pre => Data_In /= System.Null_Address and then
                 Sig_In /= System.Null_Address and then
                 Data_In_Len <= MAX_DATA_LEN and then
                 Sig_In_Len >= SIG_SIZE,
          Post => Status = STATUS_SUCCESS or Status = STATUS_ERROR;

private

   type Key_Buffer is array (1 .. 32) of Interfaces.C.unsigned_char
     with SPARK_Mode => Off;
   type Sig_Buffer is array (1 .. 2420) of Interfaces.C.unsigned_char
     with SPARK_Mode => Off;

end KMS;