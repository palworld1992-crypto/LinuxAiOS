with IDL_Types;
with Interfaces.C;
with System;

package SCC_Type_Mapper is
   pragma SPARK_Mode (On);

   procedure Map_Type
     (T : access constant IDL_Types.IDL_Type;
      Result : out Interfaces.C.size_t)
     with Export,
          Convention => C,
          External_Name => "scc_type_mapper_map_type",
          Depends => (Result => T);

   procedure Get_Alignment
     (T : access constant IDL_Types.IDL_Type;
      Align : out Interfaces.C.size_t)
     with Export,
          Convention => C,
          External_Name => "scc_type_mapper_get_alignment",
          Depends => (Align => T);

   procedure Get_Type_Name
     (T : access constant IDL_Types.IDL_Type;
      Name : out Interfaces.C.int)
     with Export,
          Convention => C,
          External_Name => "scc_type_mapper_get_type_name",
          Depends => (Name => T);

private
   pragma Export (C, Map_Type, "scc_type_mapper_map_type");
   pragma Export (C, Get_Alignment, "scc_type_mapper_get_alignment");
   pragma Export (C, Get_Type_Name, "scc_type_mapper_get_type_name");

end SCC_Type_Mapper;
