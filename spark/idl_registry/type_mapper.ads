with IDL_Types;
with Interfaces.C;

package Type_Mapper is
   pragma SPARK_Mode (On);

   -- Chuyển từ function sang procedure theo quy tắc "Procedure only" của SPARK 2026.
   -- Function với return value không được phép có side-effects trong SPARK, nhưng ánh xạ kiểu
   -- Cần thay đổi state (có thể cache kết quả). Procedure với tham số out an toàn hơn.
   procedure Map_Type
     (T : access constant IDL_Types.IDL_Type;
      Result : out Interfaces.C.size_t)
     with Export,
          Convention => C,
          External_Name => "type_mapper_map_type",
          Depends => (Result => T);

end Type_Mapper;