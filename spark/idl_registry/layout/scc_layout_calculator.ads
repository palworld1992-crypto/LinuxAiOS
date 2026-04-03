with IDL_Types;
with Interfaces.C;

package SCC_Layout_Calculator is
   pragma SPARK_Mode (On);

   procedure Compute_Layout
     (Struct : access IDL_Types.IDL_Type)
     with Export,
          Convention => C,
          External_Name => "scc_layout_calculator_compute",
          Depends => (null => Struct);

   procedure Compute_Field_Offset
     (Struct : access IDL_Types.IDL_Type;
      Field_Index : Interfaces.C.size_t;
      Offset : out Interfaces.C.size_t;
      Status : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "scc_layout_calculator_field_offset",
          Depends => (Offset | Status => (Struct, Field_Index));

   procedure Compute_Total_Size
     (Struct : access IDL_Types.IDL_Type;
      Total_Size : out Interfaces.C.size_t)
     with Export,
          Convention => C,
          External_Name => "scc_layout_calculator_total_size",
          Depends => (Total_Size => Struct);

private
   pragma Export (C, Compute_Layout, "scc_layout_calculator_compute");
   pragma Export (C, Compute_Field_Offset, "scc_layout_calculator_field_offset");
   pragma Export (C, Compute_Total_Size, "scc_layout_calculator_total_size");

end SCC_Layout_Calculator;
