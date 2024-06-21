// src/application/use_cases/fetch_content_list.rs
use crate::domain::entities::content_list::ContentList;
use crate::domain::services::content_fetching_service::ContentListFetchingService;

pub struct FetchContentList<T: ContentListFetchingService> {
    service: T,
}

impl<T: ContentListFetchingService> FetchContentList<T> {
    pub fn new(service: T) -> Self {
        Self { service }
    }

    pub fn execute(&self, directory: &str) -> Result<ContentList, String> {
        self.service.fetch_content_list(directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContentListFetchingService;

    impl ContentListFetchingService for MockContentListFetchingService {
        fn fetch_content_list(&self, _url: &str) -> Result<ContentList, String> {
            Ok(ContentList {
                uuid: todo!(), 
                name: "test.txt".to_string(), 
                url: todo!(), 
                created: todo!(), 
                modified: todo!(), 
                list_items: todo!() 
            })
        }
    }

    #[test]
    fn test_fetch_content_list() {
        let service = MockContentListFetchingService;
        let use_case = FetchContentList::new(service);
        let result = use_case.execute("dummy_directory");

        // assert_eq!(result.ok(), 1);
        // assert_eq!(result.ok(), "test.txt");
    }
}
