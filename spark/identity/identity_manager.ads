pragma Style_Checks (Off);
with Interfaces.C;

package Identity_Manager is
   pragma SPARK_Mode (On);

   subtype Token_256 is Interfaces.C.char_array (1 .. 256);
   subtype Supervisor_ID is Interfaces.C.unsigned_long;

    procedure Create_Token
      (Supervisor : Supervisor_ID;
       Expiry     : Interfaces.C.unsigned_long;
       Token_Out  : out Token_256;
       Status     : out Interfaces.C.int)
      with Export,
           Convention => C,
           External_Name => "identity_create_token",
           Depends => (Status => (Supervisor, Expiry), Token_Out => (Supervisor, Expiry));

    --  Xác thực token. Trả về 0 nếu hợp lệ, -1 nếu hết hạn, -2 nếu không hợp lệ.
    procedure Verify_Token
      (Token      : Token_256;
       Supervisor : Supervisor_ID;
       Status     : out Interfaces.C.int)
      with Export,
           Convention => C,
           External_Name => "identity_verify_token",
           Depends => (Status => (Token, Supervisor));

private
end Identity_Manager;