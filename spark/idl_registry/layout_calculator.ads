with IDL_Types;

package Layout_Calculator is
   pragma SPARK_Mode (On);

   procedure Compute_Layout
     (Struct : access IDL_Types.IDL_Type)
     with Export,
          Convention => C,
          External_Name => "layout_calculator_compute",
          Depends => (null => Struct);

end Layout_Calculator;