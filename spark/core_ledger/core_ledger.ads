with Interfaces.C;

package Core_Ledger is
   pragma SPARK_Mode (On);

   type Hash_Array is array (1 .. 32) of Interfaces.C.unsigned_char;
   type Block_Data is array (1 .. 1024) of Interfaces.C.unsigned_char;

   procedure Add_Block
     (Prev_Hash : Hash_Array;
      Data      : Block_Data;
      Data_Len  : Interfaces.C.size_t;
      New_Hash  : out Hash_Array;
      Status    : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "core_ledger_add_block",
          Depends => (New_Hash => (Prev_Hash, Data, Data_Len),
                      Status   => (Prev_Hash, Data, Data_Len));

   procedure Verify_Chain
     (Start_Index : Interfaces.C.size_t;
      End_Index   : Interfaces.C.size_t;
      Status      : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "core_ledger_verify_chain",
          Depends => (Status => (Start_Index, End_Index));

end Core_Ledger;