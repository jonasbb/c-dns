use crate::serialization::*;
use std::slice;

impl File {
    /// Iterate over all Blocks with corresponding parameters in the file.
    pub fn iter_blocks(&self) -> impl Iterator<Item = (&Block, &BlockParameters)> {
        BlockIterator {
            block_parameters: &*self.file_preamble.block_parameters,
            blocks: self.file_blocks.iter(),
        }
    }
}

/// Iterate over [`Block`]s and their parameters.
///
/// See [`File::iter_blocks`]
pub struct BlockIterator<'a> {
    pub(crate) block_parameters: &'a [BlockParameters],
    pub(crate) blocks: slice::Iter<'a, Block>,
}

impl<'a> Iterator for BlockIterator<'a> {
    type Item = (&'a Block, &'a BlockParameters);

    fn next(&mut self) -> Option<Self::Item> {
        self.blocks.next().map(|block| {
            (
                block,
                &self.block_parameters[block.block_preamble.block_parameters_index.unwrap_or(0)],
            )
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.blocks.size_hint()
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let block_parameters = self.block_parameters;
        self.blocks.fold(init, |accu, block| {
            f(
                accu,
                (
                    block,
                    &block_parameters[block.block_preamble.block_parameters_index.unwrap_or(0)],
                ),
            )
        })
    }
}

impl Block {
    /// Iterate over all Blocks with corresponding parameters in the file.
    pub fn iter_query_responses<'a>(
        &'a self,
        block_parameters: &'a BlockParameters,
    ) -> impl Iterator<
        Item = (
            &'a QueryResponse,
            Option<Timestamp>,
            &'a BlockParameters,
            &'a BlockTables,
        ),
    > {
        QueryResponseIterator {
            earliest_time: self.block_preamble.earliest_time,
            block_parameters,
            block_tables: self
                .block_tables
                .as_ref()
                .expect("Missing BlockTables in Block"),
            query_responses: self.query_responses.as_deref().unwrap_or(&[]).iter(),
        }
    }
}

/// Iterate over [`QueryResponse`]s and their parameters.
///
/// See [`Block::iter_query_responses`]
pub struct QueryResponseIterator<'a> {
    pub(crate) earliest_time: Option<Timestamp>,
    pub(crate) block_parameters: &'a BlockParameters,
    pub(crate) block_tables: &'a BlockTables,
    pub(crate) query_responses: slice::Iter<'a, QueryResponse>,
}

impl<'a> Iterator for QueryResponseIterator<'a> {
    type Item = (
        &'a QueryResponse,
        Option<Timestamp>,
        &'a BlockParameters,
        &'a BlockTables,
    );

    fn next(&mut self) -> Option<Self::Item> {
        self.query_responses.next().map(|query_response| {
            (
                query_response,
                self.earliest_time,
                self.block_parameters,
                self.block_tables,
            )
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.query_responses.size_hint()
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let earliest_time = self.earliest_time;
        let block_parameters = self.block_parameters;
        let block_tables = self.block_tables;
        self.query_responses.fold(init, |accu, query_response| {
            f(
                accu,
                (
                    query_response,
                    earliest_time,
                    block_parameters,
                    block_tables,
                ),
            )
        })
    }
}
