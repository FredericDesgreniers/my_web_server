use std::fmt::Debug;
use std::fmt::Display;

pub trait Endpoint<T: Debug, R>: Debug + Send + Sync {
    /// Strict matching will only match on perfect matches
    /// Setting this to false will allow paths that only match at the start to still match if no more precise route is available
    fn use_strict_path_matching(&self) -> bool {
        true
    }

    fn process(&self, info: RoutedInfo<T>) -> R;
}

/// Info passed by the router to the endpoint
#[derive(Debug)]
pub struct RoutedInfo<T: Debug> {
    pub data: T,
    pub path_overload: Vec<String>,
}

/// A router path is a string path (e.g. "some/router/to/somewhere") that is split at '/' and each part is represented as a series of bytes.
///
#[derive(Default, Debug)]
pub struct RouterPath {
    parts: Vec<Vec<u8>>,
}

impl From<Vec<Vec<u8>>> for RouterPath {
    fn from(data: Vec<Vec<u8>>) -> Self {
        Self { parts: data }
    }
}

impl From<&str> for RouterPath {
    /// This splits the provided `&str` at '/' and converts it to a router path
    fn from(path: &str) -> Self {
        let parts = path.split('/');

        let parts: Vec<Vec<u8>> = parts.map(|part| part.as_bytes().to_vec()).collect();

        parts.into()
    }
}

/// Router
///
/// This stores added paths in `matches`, which is a byte representation of the bytes
/// `0` bytes indicate a path boundary.
/// `router` contains the path's matching routers in the same order
///
/// This is intended to be a cache friendly router for low amounts of low-length path
///
//TODO testing needs to be done to see if this is actually faster than a string array or hashmap alternative
//TODO Since a byte comparison is made, it should be an easy simd candidate. Either it needs to verify that the compiler will generate simd for this or it should be implemented manually
//TODO Would it be simpler to use chars instead of bytes here? Does it matter, is it faster?
#[derive(Debug)]
pub struct Router<T: Debug, R> {
    endpoint: Option<Box<Endpoint<T, R>>>,
    matches: Vec<u8>,
    routers: Vec<Router<T, R>>,
}

// Debug can't be derived since T does not implement debug
impl<T: Debug, R> Default for Router<T, R> {
    fn default() -> Self {
        Self {
            endpoint: Default::default(),
            matches: Default::default(),
            routers: Default::default(),
        }
    }
}

impl<T: Debug, R> Router<T, R> {
    /// Attempt to match a path part to a router
    /// Returns `None` if no math is found
    /// Returns `Some(index)` if a match was found. This index can be used to access the match using `Router.matches`
    fn find_path_part_match(&self, path: &[u8]) -> Option<usize> {
        let mut current_match_index = 0;
        let mut current_path_index = 0;

        let mut match_result = None;

        let mut current_path = 0;

        //TODO This loop is a bit messy and does not convey meaning too well, could be re-written to be better
        // Be carefull of the <= for current_path_index, it's intentionally off by 1. This probably needs to be made better
        'match_loop: while current_path_index <= path.len()
            && current_match_index < self.matches.len()
        {
            let byte_to_match = self.matches[current_match_index];

            match byte_to_match {
                0 => {
                    // If we are at end of path boundary, check if path contains anything else to match on
                    // If not, match is a success
                    if current_path_index >= path.len() {
                        match_result = Some(current_path);
                        break 'match_loop;
                    }

                    // On '0' byte, this means we passed a path boundary and we need to restart matching on the next path
                    // We also need to reset the input path since the matching is starting from the start again
                    current_match_index += 1;
                    current_path_index = 0;

                    current_path += 1;
                    continue 'match_loop;
                }
                byte_to_check => {
                    // In the while condition, we allow `current_path_index` to go 1 past it's limit to see if theirs a 0 as a matching byte.
                    // This means that we need to check and exit if the index is out of bounds
                    if let Some(&path_byte_to_check) = path.get(current_path_index) {
                        // If their is a path mismatch, we advance past next path boundary and reset path match index
                        if byte_to_check != path_byte_to_check {
                            'skip_zeros: while let Some(match_byte) =
                                self.matches.get(current_match_index)
                            {
                                match match_byte {
                                    0 => {
                                        current_match_index += 1;
                                        current_path += 1;
                                        break 'skip_zeros;
                                    }
                                    _ => {
                                        current_match_index += 1;
                                    }
                                }
                            }

                            current_path_index = 0;
                            continue 'match_loop;
                        }
                    } else {
                        break 'match_loop;
                    }
                }
            }

            // We get here if their is a match, in that case, just continue matching
            current_match_index += 1;
            current_path_index += 1;
        }

        match_result
    }

    /// Add a path to the router that maps to a specified endpoint
    pub fn add_path(
        &mut self,
        path: impl Into<RouterPath>,
        endpoint: impl Endpoint<T, R> + 'static,
    ) {
        let mut current_router = self;
        let path = path.into();

        for part in &path.parts {
            if let Some(match_index) = current_router.find_path_part_match(part) {
                let next_router = &mut current_router.routers[match_index];
                current_router = next_router;
            } else {
                current_router.matches.extend_from_slice(&part);
                current_router.matches.push(0);

                let new_router = Router::default();
                current_router.routers.push(new_router);

                let new_router_index = current_router.routers.len() - 1;
                current_router = &mut current_router.routers[new_router_index];
            }
        }

        current_router.endpoint = Some(Box::new(endpoint));
    }

    /// Attempt to route a query to an endpoint
    /// Returns `Some(result)` if a route was found
    /// Returns `None` if no route could be found
    pub fn route(&self, path: impl Into<RouterPath>, data: T) -> Option<R> {
        let path = path.into();

        let mut current_router = self;

        // This is needed to give the path overload to the endpoint if needed
        let mut failed_to_match = false;
        let mut last_path_index = None;

        for (path_index, part) in path.parts.iter().enumerate() {
            if let Some(match_index) = current_router.find_path_part_match(part) {
                last_path_index = Some(path_index);
                current_router = &current_router.routers[match_index];
            } else {
                failed_to_match = true;
                break;
            }
        }

        if let Some(endpoint) = &current_router.endpoint {
            if endpoint.use_strict_path_matching() {
                if !failed_to_match {
                    return Some(endpoint.process(RoutedInfo {
                        data,
                        path_overload: Vec::new(),
                    }));
                }
            } else if let Some(last_path_index) = last_path_index {
                return Some(
                    endpoint.process(RoutedInfo {
                        data,
                        path_overload: path.parts[last_path_index..]
                            .into_iter()
                            .map(|part| String::from_utf8(part.to_vec()).unwrap())
                            .collect(),
                    }),
                );
            } else {
                return Some(endpoint.process(RoutedInfo {
                    data,
                    path_overload: Vec::new(),
                }));
            }
        }

        None
    }
}
