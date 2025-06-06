// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::sync::Arc;

use crate::fuzz_cases::equivalence::utils::{
    create_random_schema, generate_table_for_eq_properties, is_table_same_after_sort,
    TestScalarUDF,
};

use datafusion_common::Result;
use datafusion_expr::{Operator, ScalarUDF};
use datafusion_physical_expr::expressions::{col, BinaryExpr};
use datafusion_physical_expr::{LexOrdering, ScalarFunctionExpr};
use datafusion_physical_expr_common::sort_expr::PhysicalSortExpr;

use itertools::Itertools;

#[test]
fn test_find_longest_permutation_random() -> Result<()> {
    const N_RANDOM_SCHEMA: usize = 100;
    const N_ELEMENTS: usize = 125;
    const N_DISTINCT: usize = 5;

    for seed in 0..N_RANDOM_SCHEMA {
        // Create a random schema with random properties
        let (test_schema, eq_properties) = create_random_schema(seed as u64)?;
        // Generate a data that satisfies properties given
        let table_data_with_properties =
            generate_table_for_eq_properties(&eq_properties, N_ELEMENTS, N_DISTINCT)?;

        let test_fun = Arc::new(ScalarUDF::new_from_impl(TestScalarUDF::new()));
        let col_a = col("a", &test_schema)?;
        let floor_a = Arc::new(ScalarFunctionExpr::try_new(
            Arc::clone(&test_fun),
            vec![col_a],
            &test_schema,
        )?) as _;

        let a_plus_b = Arc::new(BinaryExpr::new(
            col("a", &test_schema)?,
            Operator::Plus,
            col("b", &test_schema)?,
        )) as _;
        let exprs = [
            col("a", &test_schema)?,
            col("b", &test_schema)?,
            col("c", &test_schema)?,
            col("d", &test_schema)?,
            col("e", &test_schema)?,
            col("f", &test_schema)?,
            floor_a,
            a_plus_b,
        ];

        for n_req in 0..=exprs.len() {
            for exprs in exprs.iter().combinations(n_req) {
                let exprs = exprs.into_iter().cloned().collect::<Vec<_>>();
                let (ordering, indices) =
                    eq_properties.find_longest_permutation(&exprs)?;
                // Make sure that find_longest_permutation return values are consistent
                let ordering2 = indices
                    .iter()
                    .zip(ordering.iter())
                    .map(|(&idx, sort_expr)| {
                        PhysicalSortExpr::new(Arc::clone(&exprs[idx]), sort_expr.options)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    ordering, ordering2,
                    "indices and lexicographical ordering do not match"
                );

                let err_msg = format!(
                    "Error in test case ordering:{ordering:?}, eq_properties: {eq_properties}"
                );
                assert_eq!(ordering.len(), indices.len(), "{err_msg}");
                // Since ordered section satisfies schema, we expect
                // that result will be same after sort (e.g sort was unnecessary).
                let Some(ordering) = LexOrdering::new(ordering) else {
                    continue;
                };
                assert!(
                    is_table_same_after_sort(ordering, &table_data_with_properties)?,
                    "{}",
                    err_msg
                );
            }
        }
    }

    Ok(())
}
